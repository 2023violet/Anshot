import init,{redact_web,preview_web} from "./pkg/shareready_e1.js";

const $=id=>document.getElementById(id);
// The V1 editor accepts locally selected images; the demo tests remain separate.
const MAX_INPUT_BYTES=25*1024*1024;
let ready=false,sourceBytes=null,sourceURL=null,label="",sourceW=0,sourceH=0;
let regions=[],selected=null,history=[],future=[],mode="mask",zoom=1;
let revision=0,output=null,stage="empty",pointer=null,nextID=1,loadTicket=0;
let copyState="not_attempted";
let nativeDownloadState="not_attempted",nativePendingUrl=null;
function revokeNativePendingUrl(){
 if(nativePendingUrl)URL.revokeObjectURL(nativePendingUrl);
 nativePendingUrl=null;
}
let spaceHeld=false, zoomMode="fit";
const HANDLE_DIRECTIONS=["nw","n","ne","e","se","s","sw","w"];
const clamp=(value,min,max)=>Math.max(min,Math.min(max,value));
const edges=r=>({left:Math.floor(Math.min(r.x0,r.x1)),top:Math.floor(Math.min(r.y0,r.y1)),
 right:Math.ceil(Math.max(r.x0,r.x1)),bottom:Math.ceil(Math.max(r.y0,r.y1))});
// Desktop host reports the *actual* native Save File dialog and disk-write
// outcome. Never show "saved" when the user canceled or WebView2 interrupted.
window.__shareReadyNativeDownload="not_attempted";
if(window.chrome?.webview?.addEventListener){
 window.chrome.webview.addEventListener("message",event=>{
  const data=event.data;
  if(!data||data.type!=="native-download")return;
  // WebView2 downloads can finish after the user has returned to editing,
  // imported a replacement source, or started a newer PNG download. Match
  // both the specific Rust-created blob URL and the active pending request.
  if(stage!=="review"||!output||output.revision!==revision||
     nativeDownloadState!=="pending"||data.uri!==nativePendingUrl)return;
  const result=String(data.state||"failed");
  if(!["saved","canceled","blocked","failed"].includes(result))return;
  nativeDownloadState=result;
  window.__shareReadyNativeDownload=result;
  revokeNativePendingUrl();
  $("download").disabled=false;
  if(result==="saved")status("Windows 已保存当前复核的 PNG。");
  else if(result==="canceled")status("保存已取消，当前成品仍可重新保存。");
  else if(result==="blocked")status("拒绝下载非本地成品；请重新检查并导出 PNG。");
  else status("保存失败：请检查路径、权限或磁盘容量后重试。");
 });
}

window.e2Debug={
 get ready(){return ready},get stage(){return stage},
 get zoom(){return zoom},get revision(){return revision},
 get outputRevision(){return output?.revision??-1},
 get regions(){return regions.map(r=>({...r}))},
 get sourceSize(){return [sourceW,sourceH]},
 get outputBytes(){return output?.bytes.byteLength??0},
 get fixture(){return label},get copyState(){return copyState},
 get nativeDownloadState(){return nativeDownloadState},
 get pendingDownloadUrl(){return nativePendingUrl},
 get outputUrl(){return output?.url??null}
};
function status(message){
 $("status").textContent=message;
 $("reviewStatus").textContent=message;
}
function snapshot(){return JSON.stringify(regions)}
function clearOutput(){
 revokeNativePendingUrl();
 if(output?.url)URL.revokeObjectURL(output.url);
 output=null;copyState="not_attempted";nativeDownloadState="not_attempted";
 window.__shareReadyNativeDownload="not_attempted";
 $("copy").disabled=true;$("download").disabled=true;
 $("outputImage").removeAttribute("src");
}
function edited(){
 revision++;clearOutput();render();
}
function saveAction(before){
 if(before!==snapshot()){
  history.push(before);if(history.length>60)history.shift();
  future=[];edited();
 }else render();
}
function undoAction(){
 if(stage!=="edit"||!history.length)return;
 future.push(snapshot());regions=JSON.parse(history.pop());selected=null;edited();
}
function redoAction(){
 if(stage!=="edit"||!future.length)return;
 history.push(snapshot());regions=JSON.parse(future.pop());selected=null;edited();
}
function render(){
 const active=document.activeElement;
 const focusedID=active?.dataset?.id;
 const focusedKind=active?.classList?.contains("mask")?"mask":
                   active?.classList?.contains("region")?"region":null;
 const editing=stage==="edit";
 $("check").disabled=!ready||!editing||!sourceBytes||regions.length===0;
 $("undo").disabled=!editing||!history.length;
 $("redo").disabled=!editing||!future.length;
 $("delete").disabled=!editing||selected===null;
 $("clearAll").disabled=!editing||regions.length===0;
 $("count").textContent=String(regions.length);
 $("regionsEmpty").hidden=regions.length>0;
 const chosen=editing?regions.find(r=>r.id===selected):null;
 $("precisionPanel").hidden=!chosen;
 $("precisionPlaceholder").hidden=Boolean(chosen);
 for(const id of ["precX","precY","precW","precH"])$(id).disabled=!chosen;
 if(chosen){
  const x=Math.floor(Math.min(chosen.x0,chosen.x1));
  const y=Math.floor(Math.min(chosen.y0,chosen.y1));
  const right=Math.ceil(Math.max(chosen.x0,chosen.x1));
  const bottom=Math.ceil(Math.max(chosen.y0,chosen.y1));
  const values={precX:x,precY:y,precW:right-x,precH:bottom-y};
  for(const [id,value] of Object.entries(values)){
   if(document.activeElement!==$(id))$(id).value=String(value);
  }
 }
 $("regionList").replaceChildren();
 for(const r of regions){
  const li=document.createElement("li");
  const button=document.createElement("button");
  button.type="button";button.className="region"+(r.id===selected?" active":"");
  button.setAttribute("aria-label","选择区域 "+r.id+"；方向键移动，Shift+方向键调整尺寸");
  button.setAttribute("aria-keyshortcuts","ArrowLeft ArrowRight ArrowUp ArrowDown Shift+ArrowLeft Shift+ArrowRight Shift+ArrowUp Shift+ArrowDown");
  button.dataset.id=String(r.id);
  button.setAttribute("aria-pressed",String(r.id===selected));
  button.setAttribute("aria-describedby","keyboardHelp");
  const sq=document.createElement("span");sq.className="square";
  const name=document.createElement("span");name.className="region-name";name.textContent="区域 "+String(r.id).padStart(2,"0");
  const size=document.createElement("span");size.className="region-size";
  size.textContent=Math.ceil(Math.abs(r.x1-r.x0))+" × "+Math.ceil(Math.abs(r.y1-r.y0));
  button.append(sq,name,size);
  button.onclick=()=>{selected=r.id;setMode("select");render();
   document.querySelector('.region.active')?.focus({preventScroll:true});};
  li.append(button);$("regionList").append(li);
 }
 const layer=$("maskLayer");layer.replaceChildren();
 for(const r of regions){
  const item=document.createElement("div");
  item.className="mask"+(r.id===selected?" selected":"");
  item.dataset.id=String(r.id);
  item.tabIndex=0;
  item.setAttribute("role","button");
  item.setAttribute("aria-label","遮挡区域 "+r.id+"；方向键移动，Shift+方向键调整尺寸");
  item.setAttribute("aria-pressed",String(r.id===selected));
  item.setAttribute("aria-describedby","keyboardHelp");
  item.setAttribute("aria-keyshortcuts","ArrowLeft ArrowRight ArrowUp ArrowDown");
  const x=Math.min(r.x0,r.x1),y=Math.min(r.y0,r.y1);
  Object.assign(item.style,{
   left:x*zoom+"px",top:y*zoom+"px",
   width:Math.max(1,Math.abs(r.x1-r.x0)*zoom)+"px",
   height:Math.max(1,Math.abs(r.y1-r.y0)*zoom)+"px"
  });
  if(r.id===selected){
   for(const dir of HANDLE_DIRECTIONS){
    const grip=document.createElement("span");
    grip.className="handle handle-"+dir;
    grip.dataset.dir=dir;
    grip.setAttribute("aria-hidden","true");
    grip.title="拖动 "+dir.toUpperCase()+" 手柄调整遮挡边界";
    item.append(grip);
   }
  }
  layer.append(item);
 }
 $("zoomValue").textContent=Math.round(zoom*100)+"%";
 // Rebuilding the region nodes must not discard keyboard focus.
 if(editing&&focusedKind&&focusedID){
  const byKind=kind=>document.querySelector((kind==="mask"?'.mask[data-id="':'.region[data-id="')+focusedID+'"]');
  const candidate=byKind(focusedKind);
  (candidate?.getClientRects().length?candidate:byKind("mask"))?.focus({preventScroll:true});
 }
}
function commitPrecision(){
 if(stage!=="edit"||selected===null)return;
 const r=regions.find(item=>item.id===selected);if(!r)return;
 const read=id=>Number($(id).value);
 const [x,y,w,h]=["precX","precY","precW","precH"].map(read);
 if(![x,y,w,h].every(Number.isSafeInteger)||x<0||y<0||w<1||h<1||x>=sourceW||y>=sourceH){
  status("坐标必须为图内整数，宽度和高度至少 1 像素");render();return;
 }
 const width=Math.min(w,sourceW-x),height=Math.min(h,sourceH-y);
 const before=snapshot();
 Object.assign(r,{x0:x,y0:y,x1:x+width,y1:y+height});
 if(before!==snapshot()){
  saveAction(before);
  status("已精确设置遮挡位置："+x+", "+y+" · "+width+" × "+height+" 像素");
 }else render();
}
function updatePanCursor(){
 const panning=mode==="pan"||spaceHeld;
 $("viewport").classList.toggle("can-pan",panning);
 $("viewport").classList.toggle("is-panning",pointer?.kind==="pan");
 $("frame").style.cursor=panning?"grab":mode==="select"?"default":"crosshair";
}
function setMode(newMode){
 mode=newMode;
 for(const b of document.querySelectorAll("[data-tool]")){
  const active=b.dataset.tool===mode;
  b.classList.toggle("active",active);
  b.setAttribute("aria-pressed",String(active));
 }
 updatePanCursor();
}
function setZoom(value,anchorX=null,anchorY=null,preserveFit=false){
 if(!sourceW)return;
 const vp=$("viewport"),frame=$("frame");
 const rect=frame.getBoundingClientRect();
 const vrect=vp.getBoundingClientRect();
 const ax=anchorX??vrect.left+vrect.width/2;
 const ay=anchorY??vrect.top+vrect.height/2;
 const relativeX=rect.width>0?(ax-rect.left)/rect.width:0.5;
 const relativeY=rect.height>0?(ay-rect.top)/rect.height:0.5;
 const next=clamp(value,.08,2.5);
 zoomMode=preserveFit?"fit":"manual";
 const unchanged=Math.abs(next-zoom)<1e-9;
 zoom=next;
 // Always initialize both dimensions. The very first imported image often
 // fits at 100%; skipping styles for an unchanged zoom leaves an auto-sized
 // frame stretched to the viewport and destroys source-pixel hit testing.
 frame.style.width=sourceW*zoom+"px";
 frame.style.height=sourceH*zoom+"px";
 render();
 if(unchanged)return;
 const after=frame.getBoundingClientRect();
 vp.scrollLeft+=after.left+relativeX*after.width-ax;
 vp.scrollTop+=after.top+relativeY*after.height-ay;
}
function fit(){
 if(!sourceW)return;
 const vp=$("viewport");
 setZoom(Math.min(1,Math.max(.08,(vp.clientWidth-100)/sourceW),Math.max(.08,(vp.clientHeight-120)/sourceH)),null,null,true);
}
new ResizeObserver(()=>{
 if(zoomMode==="fit"&&sourceW&&stage==="edit")fit();
}).observe($("viewport"));
async function decodeImage(url){
 const image=new Image();image.src=url;await image.decode();return image;
}
async function loadFile(file){
 if(!ready){status("Rust/WASM 尚未加载完成");return}
 if(!file)return;

 const ticket=++loadTicket;
 status("正在本地读取并解码图片…");
 try{
  if(file.size===0||file.size>MAX_INPUT_BYTES)throw Error("图片必须大于 0 且不超过 25 MiB");
  const bytes=new Uint8Array(await file.arrayBuffer());
  if(ticket!==loadTicket)return;
  const isPNG=bytes.length>=8&&bytes[0]===137&&bytes[1]===80&&bytes[2]===78&&bytes[3]===71;
  const isJPEG=bytes.length>=3&&bytes[0]===255&&bytes[1]===216&&bytes[2]===255;
  if(!isPNG&&!isJPEG)throw Error("仅支持有效的 PNG/JPEG 图片");
  // Rust checks encoded size, decoded pixel budget and EXIF orientation.
  // The edit preview and final result share normalized source pixel geometry.
  const preview=preview_web(bytes);
  const url=URL.createObjectURL(new Blob([preview],{type:"image/png"}));
  let image;
  try{image=await decodeImage(url)}catch(e){URL.revokeObjectURL(url);throw e}
  if(ticket!==loadTicket){URL.revokeObjectURL(url);return}
  if(!image.naturalWidth||!image.naturalHeight||image.naturalWidth*image.naturalHeight>24000000){
   URL.revokeObjectURL(url);throw Error("解码图像超限");
  }
  // Validate the replacement before asking the user to discard old masks.
  // A bad/corrupted file must not trigger a destructive confirmation dialog.
  if((stage==="edit"||stage==="review")&&regions.length&&!confirm("更换图片会清空当前遮挡。是否继续？")){
   URL.revokeObjectURL(url);return;
  }
  const previousStage=stage;
  // A new verified source invalidates the previous review surface, including
  // its modal focus trap. Keep prior source/edit state when import rejects.
  $("review").classList.remove("visible");
  document.querySelector(".app").inert=false;
  if(sourceURL)URL.revokeObjectURL(sourceURL);
  sourceURL=url;sourceBytes=bytes;
  label=(file.name||"粘贴图片").replace(/[\\/\x00-\x1f]/g,"_").slice(0,120);
  sourceW=image.naturalWidth;sourceH=image.naturalHeight;
  regions=[];selected=null;history=[];future=[];nextID=1;
  revision++;clearOutput();
  stage="edit";$("source").src=url;$("frame").style.display="block";$("empty").style.display="none";
  $("filename").textContent=label;$("imageName").textContent=label;
  $("dimensions").textContent=sourceW+" × "+sourceH;$("format").textContent=isPNG?"PNG":"JPEG";
  fit();status("图片已在本机解码并校正 JPEG 方向。请手动框选要隐藏的内容。");
  if(previousStage==="review")$("createMask").focus({preventScroll:true});
 }catch(e){if(ticket===loadTicket)status("导入失败（原有编辑已保留）："+String(e?.message||e))}
}
const frame=$("frame"),viewport=$("viewport");
// Ctrl/Cmd+wheel zooms around the pointer in original image coordinates.
viewport.addEventListener("wheel",e=>{
 if(stage!=="edit"||!sourceW||!(e.ctrlKey||e.metaKey))return;
 e.preventDefault();
 setZoom(zoom*(e.deltaY<0?1.12:1/1.12),e.clientX,e.clientY);
},{passive:false});
function coordinates(e){
 const box=frame.getBoundingClientRect();
 // Use the actual rendered pixel scale, not merely app zoom: browser
 // page zoom, DPR and ancestor CSS transforms may scale the frame too.
 const sx=box.width/sourceW,sy=box.height/sourceH;
 return {x:Math.max(0,Math.min(sourceW,(e.clientX-box.left)/sx)),
         y:Math.max(0,Math.min(sourceH,(e.clientY-box.top)/sy))};
}
// Pan can start on the checkerboard OUTSIDE the image. Middle-mouse, H
// and held Space temporarily pan without modifying pixel masks.
viewport.addEventListener("pointerdown",e=>{
 if(stage!=="edit"||!sourceW||pointer)return;
 if(!(e.button===1||(e.button===0&&(mode==="pan"||spaceHeld))))return;
 e.preventDefault();e.stopPropagation();
 pointer={kind:"pan",pointerId:e.pointerId,clientX:e.clientX,clientY:e.clientY,
          left:viewport.scrollLeft,top:viewport.scrollTop};
 viewport.setPointerCapture(e.pointerId);
 updatePanCursor();
},true);
viewport.addEventListener("pointermove",e=>{
 if(pointer?.kind!=="pan"||pointer.pointerId!==e.pointerId)return;
 e.preventDefault();e.stopPropagation();
 viewport.scrollLeft=pointer.left-(e.clientX-pointer.clientX);
 viewport.scrollTop=pointer.top-(e.clientY-pointer.clientY);
},true);
function finishPan(e){
 if(pointer?.kind!=="pan"||pointer.pointerId!==e.pointerId)return;
 e.stopPropagation();
 pointer=null;
 if(viewport.hasPointerCapture(e.pointerId))viewport.releasePointerCapture(e.pointerId);
 updatePanCursor();
}
viewport.addEventListener("pointerup",finishPan,true);
viewport.addEventListener("pointercancel",finishPan,true);
function assignEdges(r,left,top,right,bottom){
 // Never flip or create zero-pixel masks: Rust export always receives a
 // rectangle with >=1 source pixel in both directions.
 Object.assign(r,{x0:left,y0:top,x1:right,y1:bottom});
}
frame.addEventListener("pointerdown",e=>{
 if(stage!=="edit"||e.button!==0||pointer||mode==="pan"||spaceHeld)return;
 e.preventDefault();
 const p=coordinates(e),target=e.target.closest(".mask"),before=snapshot();
 if(target){
  const id=Number(target.dataset.id),r=regions.find(x=>x.id===id);
  if(!r)return;
  selected=id;
  const dir=e.target.closest(".handle")?.dataset.dir;
  pointer={kind:dir?"resize":"move",dir:dir||null,id,pointerId:e.pointerId,
           from:p,original:edges(r),before};
 }else if(mode==="mask"){
  const x=Math.round(p.x),y=Math.round(p.y);
  const r={id:nextID++,x0:x,y0:y,x1:x,y1:y};
  regions.push(r);selected=r.id;
  pointer={kind:"draw",id:r.id,pointerId:e.pointerId,from:p,original:{...r},before};
 }else{
  selected=null;render();return;
 }
 frame.setPointerCapture(e.pointerId);render();
});
frame.addEventListener("pointermove",e=>{
 if(!pointer||pointer.kind==="pan"||pointer.pointerId!==e.pointerId||stage!=="edit")return;
 const p=coordinates(e),r=regions.find(x=>x.id===pointer.id);
 if(!r)return;
 const o=pointer.original;
 if(pointer.kind==="draw"){
  r.x1=Math.round(p.x);r.y1=Math.round(p.y);
 }else if(pointer.kind==="resize"){
  let {left,top,right,bottom}=o;
  const dir=pointer.dir;
  const x=clamp(Math.round(p.x),0,sourceW),y=clamp(Math.round(p.y),0,sourceH);
  if(dir.includes("w"))left=clamp(x,0,right-1);
  if(dir.includes("e"))right=clamp(x,left+1,sourceW);
  if(dir.includes("n"))top=clamp(y,0,bottom-1);
  if(dir.includes("s"))bottom=clamp(y,top+1,sourceH);
  assignEdges(r,left,top,right,bottom);
 }else if(pointer.kind==="move"){
  const w=o.right-o.left,h=o.bottom-o.top;
  const dx=Math.round(p.x-pointer.from.x),dy=Math.round(p.y-pointer.from.y);
  const left=clamp(o.left+dx,0,sourceW-w);
  const top=clamp(o.top+dy,0,sourceH-h);
  assignEdges(r,left,top,left+w,top+h);
 }
 render();
});
frame.addEventListener("pointerup",e=>{
 if(!pointer||pointer.kind==="pan"||pointer.pointerId!==e.pointerId)return;
 const g=pointer;pointer=null;
 const r=regions.find(x=>x.id===g.id);
 if(g.kind==="draw"&&r&&(!Math.abs(r.x1-r.x0)||!Math.abs(r.y1-r.y0))){
  regions=JSON.parse(g.before);selected=null;render();return;
 }
 saveAction(g.before);
});
frame.addEventListener("pointercancel",e=>{
 if(!pointer||pointer.kind==="pan"||pointer.pointerId!==e.pointerId)return;
 if(pointer.before)regions=JSON.parse(pointer.before);
 pointer=null;render();
});
function nudgeSelected(key,resize){
 if(stage!=="edit"||selected===null)return false;
 const r=regions.find(x=>x.id===selected);
 if(!r)return false;
 const delta={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]}[key];
 if(!delta)return false;
 const [dx,dy]=delta,before=snapshot();
 const keyboardFocusKind=document.activeElement?.classList.contains("mask")?"mask":"region";
 let left=Math.floor(Math.min(r.x0,r.x1)),top=Math.floor(Math.min(r.y0,r.y1));
 let right=Math.ceil(Math.max(r.x0,r.x1)),bottom=Math.ceil(Math.max(r.y0,r.y1));
 if(resize){
  right=Math.max(left+1,Math.min(sourceW,right+dx));
  bottom=Math.max(top+1,Math.min(sourceH,bottom+dy));
 }else{
  const w=right-left,h=bottom-top;
  left=Math.max(0,Math.min(sourceW-w,left+dx));
  top=Math.max(0,Math.min(sourceH-h,top+dy));
  right=left+w;bottom=top+h;
 }
 r.x0=left;r.y0=top;r.x1=right;r.y1=bottom;
 saveAction(before);
 // Render creates fresh DOM nodes; keep keyboard focus on this region.
 const activeID=selected;
 queueMicrotask(()=>{
  const region=document.querySelector('.region[data-id="'+activeID+'"]');
  const mask=document.querySelector('.mask[data-id="'+activeID+'"]');
  // In compact layout the inspector and its region controls are display:none.
  // Focusing the hidden node silently drops subsequent keyboard operations.
  const inspectorVisible=getComputedStyle($("regionList").closest(".panel")).display!=="none";
  const focusNode=keyboardFocusKind==="mask"||!inspectorVisible?(mask||region):(region||mask);
  focusNode?.focus({preventScroll:true});
 });
 status(resize?"已使用键盘调整遮挡尺寸":"已使用键盘移动遮挡区域");
 return true;
}
function createKeyboardMask(){
 if(!ready||stage!=="edit"||!sourceW||!sourceH)return;
 const before=snapshot(),width=Math.min(180,sourceW),height=Math.min(64,sourceH);
 const box=frame.getBoundingClientRect(),view=viewport.getBoundingClientRect();
 const centerX=(view.left+view.width/2-box.left)/(box.width/sourceW);
 const centerY=(view.top+view.height/2-box.top)/(box.height/sourceH);
 const x=Math.round(Math.max(0,Math.min(sourceW-width,centerX-width/2)));
 const y=Math.round(Math.max(0,Math.min(sourceH-height,centerY-height/2)));
 const r={id:nextID++,x0:x,y0:y,x1:x+width,y1:y+height};
 regions.push(r);selected=r.id;
 setMode("select");saveAction(before);
 document.querySelector('.mask[data-id="'+r.id+'"]')?.focus({preventScroll:true});
 status("已通过键盘创建区域 "+r.id+"（"+width+" × "+height+" 像素）。方向键移动，Shift＋方向键调整尺寸。");
}
function clearAllMasks(){
 if(stage!=="edit"||regions.length===0)return;
 if(!confirm("清空全部 "+regions.length+" 个遮挡区域？该操作可以撤销。"))return;
 const before=snapshot();
 regions=[];selected=null;saveAction(before);
 $("createMask").focus({preventScroll:true});
 status("已清空全部遮挡。可按 Ctrl+Z 撤销。");
}
function deleteSelected(){
 if(stage!=="edit"||selected===null)return;
 const before=snapshot();regions=regions.filter(r=>r.id!==selected);selected=null;saveAction(before);
 $("createMask").focus({preventScroll:true});
 status("遮挡区域已删除；可按 N 创建新区域。");
}
async function enterReview(){
 if(stage!=="edit"||!ready||!sourceBytes||regions.length===0){
  status("请先绘制有效遮挡区域");return;
 }
 const v=revision,ticket=loadTicket;
 stage="render";render();status("正在由 Rust/WASM 生成新的 PNG…");
 try{
  const rects=regions.flatMap(r=>[
   Math.floor(Math.min(r.x0,r.x1)),Math.floor(Math.min(r.y0,r.y1)),
   Math.ceil(Math.max(r.x0,r.x1)),Math.ceil(Math.max(r.y0,r.y1))
  ]);
  const bytes=redact_web(sourceBytes,new Int32Array(rects));
  const blob=new Blob([bytes],{type:"image/png"});
  const url=URL.createObjectURL(blob);
  let image;
  try{image=await decodeImage(url)}catch(e){URL.revokeObjectURL(url);throw e}
  if(revision!==v||loadTicket!==ticket){URL.revokeObjectURL(url);throw Error("异步生成结果已经失效")}
  if(image.naturalWidth!==sourceW||image.naturalHeight!==sourceH){
   URL.revokeObjectURL(url);throw Error("输出尺寸不一致");
  }
  clearOutput();output={bytes,blob,url,revision:v};
  $("outputImage").src=url;
  $("reviewSummary").textContent="已手动遮挡 "+regions.length+" 处。请检查未覆盖内容。";
  $("copy").disabled=false;$("download").disabled=false;
  $("review").classList.add("visible");
  stage="review";status("当前图片是 Rust 新编码的 PNG，已完成独立解码复核。");
  document.querySelector(".app").inert=true;
  $("back").focus({preventScroll:true});
 }catch(e){
  // Late results must not replace a newer image/edit/review state.
  if(stage==="render")stage="edit";
  if(revision===v && loadTicket===ticket){
   status("生成失败，拒绝导出原图："+String(e?.message||e));
  }
 }finally{render()}
}
function back(){
 if(stage!=="review")return;
 $("review").classList.remove("visible");stage="edit";
 document.querySelector(".app").inert=false;
 render();$("check").focus({preventScroll:true});
}
function download(){
 if(stage!=="review"||!output||output.revision!==revision){
  status("输出已过期，请重新检查");return;
 }
 const inDesktop=Boolean(window.chrome?.webview);
 if(inDesktop&&nativeDownloadState==="pending")return;
 // WebView2 can suppress a second download of a previously canceled
 // blob: URL. Each attempt gets a NEW one-use URL pointing at the same
 // already-reviewed Rust-generated PNG bytes. No re-encoding or raw fallback.
 const attemptUrl=URL.createObjectURL(output.blob);
 const a=document.createElement("a");
 const stem=label.replace(/\.(png|jpe?g)$/i,"").replace(/[^\w\u4e00-\u9fff-]+/g,"_").slice(0,60)||"image";
 a.href=attemptUrl;a.download=stem+"_share_ready.png";
 if(inDesktop){
  nativePendingUrl=attemptUrl;
  nativeDownloadState="pending";
  window.__shareReadyNativeDownload="pending";
  $("download").disabled=true;
 }
 try{
  a.click();
  if(!inDesktop)setTimeout(()=>URL.revokeObjectURL(attemptUrl),60000);
  status(inDesktop?
   "Windows 正在等待保存结果；取消或失败后可以重试。":
   "已请求浏览器下载当前 Rust PNG。");
 }catch(e){
  if(inDesktop){
   revokeNativePendingUrl();
   nativeDownloadState="failed";window.__shareReadyNativeDownload="failed";
   $("download").disabled=false;
  }else URL.revokeObjectURL(attemptUrl);
  status("无法启动保存："+String(e?.message||e)+"。请重试。");
 }
}
async function copy(){
 if(stage!=="review"||!output||output.revision!==revision){
  status("输出已过期，请重新检查");return;
 }
 const source=output,atRevision=revision,atLoad=loadTicket;
 const stale=()=>stage!=="review"||output!==source||revision!==atRevision||
                  loadTicket!==atLoad||source.revision!==revision;
 $("copy").disabled=true;
 try{
  if(!navigator.clipboard?.write||!window.ClipboardItem)throw Error("不支持 PNG Clipboard API");
  await navigator.clipboard.write([new ClipboardItem({"image/png":source.blob})]);
  if(stale())return; // Never overwrite a newer review's state or UI.
  copyState="write_succeeded_readback_unverified";
  status("PNG 写入剪贴板成功；尚未验证实际粘贴回读。");
 }catch(e){
  if(stale())return; // A failed old copy must not taint the current image.
  copyState="denied_or_unavailable";
  status("复制失败："+String(e?.message||e)+"。请下载 PNG。");
 }finally{
  if(!stale())$("copy").disabled=false;
 }
}
for(const b of document.querySelectorAll("[data-tool]"))b.onclick=()=>setMode(b.dataset.tool);
$("undo").onclick=undoAction;$("redo").onclick=redoAction;$("delete").onclick=deleteSelected;
$("clearAll").onclick=clearAllMasks;
$("createMask").onclick=createKeyboardMask;
$("zoomIn").onclick=()=>setZoom(zoom*1.25);
$("zoomOut").onclick=()=>setZoom(zoom/1.25);
$("zoomFit").onclick=fit;
$("zoomActual").onclick=()=>setZoom(1);
for(const id of ["precX","precY","precW","precH"]){
 $(id).addEventListener("change",commitPrecision);
 $(id).addEventListener("keydown",event=>{
  if(event.key==="Enter"){event.preventDefault();$(id).blur()}
 });
}
$("check").onclick=enterReview;$("back").onclick=back;
$("download").onclick=download;$("copy").onclick=copy;
function choose(){$("pick").click()}
$("chooseTop").onclick=choose;$("chooseEmpty").onclick=choose;
$("pick").onchange=()=>{
 const file=$("pick").files?.[0];if(file)loadFile(file);$("pick").value="";
};
$("loadDemo").onclick=choose;
document.addEventListener("dragover",e=>{
 if(e.dataTransfer?.types?.includes("Files")){
  e.preventDefault();e.dataTransfer.dropEffect="copy";$("dropzone").classList.add("dragging");
 }
});
document.addEventListener("dragleave",e=>{
 if(!e.relatedTarget)$("dropzone").classList.remove("dragging");
});
document.addEventListener("drop",e=>{
 if(e.dataTransfer?.files?.length){
  e.preventDefault();$("dropzone").classList.remove("dragging");
  loadFile(e.dataTransfer.files[0]);
 }
});
document.addEventListener("paste",e=>{
 if(e.target instanceof Element && e.target.closest("input,textarea,[contenteditable]"))return;
 const item=Array.from(e.clipboardData?.items||[]).find(x=>x.type.startsWith("image/"));
 if(!item)return;e.preventDefault();
 const file=item.getAsFile();if(file)loadFile(file);
});
document.addEventListener("keydown",e=>{
 if(e.target.closest("input,textarea,[contenteditable]"))return;
 if(e.code==="Space"&&stage==="edit"&&!e.target.closest("button,[role=button]")){
  e.preventDefault();spaceHeld=true;updatePanCursor();return;
 }
 const k=e.key.toLowerCase();
 if(e.key==="Escape"&&stage==="review"){back();return}
 if((e.ctrlKey||e.metaKey)&&k==="z"){e.preventDefault();if(e.shiftKey)redoAction();else undoAction();return}
 if((e.ctrlKey||e.metaKey)&&k==="y"){e.preventDefault();redoAction();return}
 if(stage!=="edit")return;
 if(k==="n"&&!e.ctrlKey&&!e.metaKey&&!e.altKey&&!e.shiftKey){
  e.preventDefault();createKeyboardMask();return;
 }
 const regionTarget=e.target.closest("#regionList .region, #maskLayer .mask");
 if(regionTarget&&(e.key==="Enter"||e.key===" ")&&!e.ctrlKey&&!e.metaKey){
  e.preventDefault();selected=Number(regionTarget.dataset.id);setMode("select");render();
  status("已选中遮挡区域 "+selected+"。方向键移动，Shift＋方向键调整尺寸。");
  return;
 }
 if(regionTarget&&e.key.startsWith("Arrow")&&!e.ctrlKey&&!e.metaKey){
  selected=Number(regionTarget.dataset.id);
  e.preventDefault();
  nudgeSelected(e.key,e.shiftKey);
  return;
 }
 if(k==="r")setMode("mask");
 if(k==="v")setMode("select");
 if(k==="h")setMode("pan");
 if(k==="0")fit();
 if(k==="1"&&!e.ctrlKey&&!e.metaKey&&!e.altKey&&!e.shiftKey)setZoom(1);
 if(e.key==="Delete"||e.key==="Backspace")deleteSelected();
});
document.addEventListener("keyup",e=>{
 if(e.code==="Space"&&spaceHeld){spaceHeld=false;updatePanCursor()}
});
window.addEventListener("blur",()=>{spaceHeld=false;updatePanCursor()});
try{
 await init();ready=true;
 status("ShareReady 已就绪，选择、拖放或粘贴本地 PNG/JPEG 开始。");
}catch(e){status("Rust/WASM 初始化失败："+String(e?.message||e))}
setMode("mask");render();