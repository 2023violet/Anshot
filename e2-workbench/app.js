import init,{redact_web} from "../e1-spike/web/pkg/shareready_e1.js";

const $=id=>document.getElementById(id);
const FIXTURE_SHA={
 "32eb61e571e11405e83619863ea0070da732ef04df8c40b519e0a7865626d5d6":"T01_CHAT",
 "2ccad22113adb43caf017818d1fe80245ea9cb765901978d89e092ef65c89eb2":"T02_ORDER",
 "b50f6070f4704ef7753ba58493b8e629b51507723ac4c4c8ccff9b629461f83d":"T04_EDGES",
 "97466f35a057c671195c3ec8902c941336e5d61fd24648abe3abe6698142ce95":"T05_JPEG_EXIF"
};
let ready=false,sourceBytes=null,sourceURL=null,label="",sourceW=0,sourceH=0;
let regions=[],selected=null,history=[],future=[],mode="mask",zoom=1;
let revision=0,output=null,stage="empty",pointer=null,nextID=1,loadTicket=0;

window.e2Debug={
 get ready(){return ready},get stage(){return stage},
 get zoom(){return zoom},get revision(){return revision},
 get outputRevision(){return output?.revision??-1},
 get regions(){return regions.map(r=>({...r}))},
 get sourceSize(){return [sourceW,sourceH]},
 get outputBytes(){return output?.bytes.byteLength??0},
 get fixture(){return label}
};
function status(message){$("status").textContent=message}
function snapshot(){return JSON.stringify(regions)}
function clearOutput(){
 if(output?.url)URL.revokeObjectURL(output.url);
 output=null;$("copy").disabled=true;$("download").disabled=true;
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
 const editing=stage==="edit";
 $("check").disabled=!ready||!editing||!sourceBytes||regions.length===0;
 $("undo").disabled=!editing||!history.length;
 $("redo").disabled=!editing||!future.length;
 $("delete").disabled=!editing||selected===null;
 $("count").textContent=String(regions.length);
 $("regionsEmpty").hidden=regions.length>0;
 $("regionList").replaceChildren();
 for(const r of regions){
  const li=document.createElement("li");
  const button=document.createElement("button");
  button.type="button";button.className="region"+(r.id===selected?" active":"");
  button.setAttribute("aria-label","选择区域 "+r.id);
  const sq=document.createElement("span");sq.className="square";
  const name=document.createElement("span");name.className="region-name";name.textContent="区域 "+String(r.id).padStart(2,"0");
  const size=document.createElement("span");size.className="region-size";
  size.textContent=Math.ceil(Math.abs(r.x1-r.x0))+" × "+Math.ceil(Math.abs(r.y1-r.y0));
  button.append(sq,name,size);
  button.onclick=()=>{selected=r.id;setMode("select");render()};
  li.append(button);$("regionList").append(li);
 }
 const layer=$("maskLayer");layer.replaceChildren();
 for(const r of regions){
  const item=document.createElement("div");
  item.className="mask"+(r.id===selected?" selected":"");
  item.dataset.id=String(r.id);
  const x=Math.min(r.x0,r.x1),y=Math.min(r.y0,r.y1);
  Object.assign(item.style,{
   left:x*zoom+"px",top:y*zoom+"px",
   width:Math.max(1,Math.abs(r.x1-r.x0)*zoom)+"px",
   height:Math.max(1,Math.abs(r.y1-r.y0)*zoom)+"px"
  });
  if(r.id===selected){
   const grip=document.createElement("span");grip.className="handle";
   grip.title="拖动调整矩形大小";item.append(grip);
  }
  layer.append(item);
 }
 $("zoomValue").textContent=Math.round(zoom*100)+"%";
}
function setMode(newMode){
 mode=newMode;
 for(const b of document.querySelectorAll("[data-tool]")){
  const active=b.dataset.tool===mode;
  b.classList.toggle("active",active);
  b.setAttribute("aria-pressed",String(active));
 }
 $("frame").style.cursor=mode==="pan"?"grab":mode==="select"?"default":"crosshair";
}
function setZoom(value){
 if(!sourceW)return;
 zoom=Math.max(.08,Math.min(2.5,value));
 $("frame").style.width=sourceW*zoom+"px";
 $("frame").style.height=sourceH*zoom+"px";
 render();
}
function fit(){
 if(!sourceW)return;
 const vp=$("viewport");
 setZoom(Math.min(1,Math.max(.08,(vp.clientWidth-100)/sourceW),Math.max(.08,(vp.clientHeight-120)/sourceH)));
}
async function sha256(bytes){
 const digest=new Uint8Array(await crypto.subtle.digest("SHA-256",bytes));
 return Array.from(digest).map(x=>x.toString(16).padStart(2,"0")).join("");
}
async function decodeImage(url){
 const image=new Image();image.src=url;await image.decode();return image;
}
async function loadFile(file){
 if(!ready){status("Rust/WASM 尚未加载完成");return}
 if(stage==="edit"&&regions.length&&!confirm("更换图片会清空当前遮挡。是否继续？"))return;
 const ticket=++loadTicket;
 try{
  const bytes=new Uint8Array(await file.arrayBuffer());
  if(bytes.byteLength>25*1024*1024)throw Error("输入大小超过测试上限");
  const key=FIXTURE_SHA[await sha256(bytes)];
  if(!key)throw Error("只能使用冻结的 T01/T02/T04/T05 合成测试图片，不能导入私人截图");
  const isPNG=bytes[0]===137&&bytes[1]===80&&bytes[2]===78&&bytes[3]===71;
  const isJPEG=bytes[0]===255&&bytes[1]===216&&bytes[2]===255;
  if(!isPNG&&!isJPEG)throw Error("仅支持 PNG/JPEG");
  const url=URL.createObjectURL(new Blob([bytes],{type:isPNG?"image/png":"image/jpeg"}));
  let image;
  try{image=await decodeImage(url)}catch(e){URL.revokeObjectURL(url);throw e}
  if(ticket!==loadTicket){URL.revokeObjectURL(url);return}
  if(!image.naturalWidth||!image.naturalHeight||image.naturalWidth*image.naturalHeight>24000000){
   URL.revokeObjectURL(url);throw Error("解码图像超限");
  }
  if(sourceURL)URL.revokeObjectURL(sourceURL);
  sourceURL=url;sourceBytes=bytes;label=key;
  sourceW=image.naturalWidth;sourceH=image.naturalHeight;
  regions=[];selected=null;history=[];future=[];nextID=1;
  revision++;clearOutput();
  stage="edit";$("source").src=url;$("frame").style.display="block";$("empty").style.display="none";
  $("filename").textContent=key;$("imageName").textContent=key;
  $("dimensions").textContent=sourceW+" × "+sourceH;$("format").textContent=isPNG?"PNG":"JPEG";
  fit();status("已导入合成图片。请绘制需要遮挡的矩形。");
 }catch(e){status("导入失败："+String(e?.message||e))}
}
const frame=$("frame"),viewport=$("viewport");
function coordinates(e){
 const box=frame.getBoundingClientRect();
 return {x:Math.max(0,Math.min(sourceW,(e.clientX-box.left)/zoom)),
         y:Math.max(0,Math.min(sourceH,(e.clientY-box.top)/zoom))};
}
frame.addEventListener("pointerdown",e=>{
 if(stage!=="edit"||e.button!==0)return;
 e.preventDefault();
 const p=coordinates(e);
 if(mode==="pan"){
  pointer={kind:"pan",clientX:e.clientX,clientY:e.clientY,left:viewport.scrollLeft,top:viewport.scrollTop};
  frame.setPointerCapture(e.pointerId);return;
 }
 const box=e.target.closest(".mask"),before=snapshot();
 if(box){
  const id=Number(box.dataset.id),r=regions.find(x=>x.id===id);
  selected=id;
  pointer={kind:e.target.classList.contains("handle")?"resize":"move",id,from:p,original:{...r},before};
 }else if(mode==="mask"){
  const r={id:nextID++,x0:p.x,y0:p.y,x1:p.x,y1:p.y};
  regions.push(r);selected=r.id;
  pointer={kind:"draw",id:r.id,from:p,original:{...r},before};
 }else{
  selected=null;render();return;
 }
 frame.setPointerCapture(e.pointerId);render();
});
frame.addEventListener("pointermove",e=>{
 if(!pointer||stage!=="edit")return;
 if(pointer.kind==="pan"){
  viewport.scrollLeft=pointer.left-(e.clientX-pointer.clientX);
  viewport.scrollTop=pointer.top-(e.clientY-pointer.clientY);return;
 }
 const p=coordinates(e),r=regions.find(x=>x.id===pointer.id);
 if(!r)return;
 const o=pointer.original;
 if(pointer.kind==="draw"){r.x1=p.x;r.y1=p.y}
 else if(pointer.kind==="resize"){r.x1=p.x;r.y1=p.y}
 else if(pointer.kind==="move"){
  const w=Math.abs(o.x1-o.x0),h=Math.abs(o.y1-o.y0);
  const x=Math.max(0,Math.min(sourceW-w,Math.min(o.x0,o.x1)+p.x-pointer.from.x));
  const y=Math.max(0,Math.min(sourceH-h,Math.min(o.y0,o.y1)+p.y-pointer.from.y));
  r.x0=x;r.y0=y;r.x1=x+w;r.y1=y+h;
 }
 render();
});
frame.addEventListener("pointerup",()=>{
 if(!pointer)return;const g=pointer;pointer=null;
 if(g.kind==="pan")return;
 const r=regions.find(x=>x.id===g.id);
 if(g.kind==="draw"&&r&&(Math.abs(r.x1-r.x0)<.5||Math.abs(r.y1-r.y0)<.5)){
  regions=JSON.parse(g.before);selected=null;render();return;
 }
 saveAction(g.before);
});
frame.addEventListener("pointercancel",()=>{
 if(pointer?.before)regions=JSON.parse(pointer.before);
 pointer=null;render();
});
function deleteSelected(){
 if(stage!=="edit"||selected===null)return;
 const before=snapshot();regions=regions.filter(r=>r.id!==selected);selected=null;saveAction(before);
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
 }catch(e){
  stage="edit";status("生成失败，拒绝导出原图："+String(e?.message||e));
 }finally{render()}
}
function back(){
 if(stage!=="review")return;
 $("review").classList.remove("visible");stage="edit";render();
}
function download(){
 if(stage!=="review"||!output||output.revision!==revision){
  status("输出已过期，请重新检查");return;
 }
 const a=document.createElement("a");
 a.href=output.url;a.download="shareready_"+label.toLowerCase()+".png";
 a.click();status("已请求浏览器下载当前 Rust PNG。");
}
async function copy(){
 if(stage!=="review"||!output||output.revision!==revision){
  status("输出已过期，请重新检查");return;
 }
 try{
  if(!navigator.clipboard?.write||!window.ClipboardItem)throw Error("不支持 PNG Clipboard API");
  await navigator.clipboard.write([new ClipboardItem({"image/png":output.blob})]);
  status("PNG 已写入剪贴板；实际粘贴回读尚未验证。");
 }catch(e){
  status("复制失败："+String(e?.message||e)+"。请下载 PNG。");
 }
}
for(const b of document.querySelectorAll("[data-tool]"))b.onclick=()=>setMode(b.dataset.tool);
$("undo").onclick=undoAction;$("redo").onclick=redoAction;$("delete").onclick=deleteSelected;
$("zoomIn").onclick=()=>setZoom(zoom*1.25);
$("zoomOut").onclick=()=>setZoom(zoom/1.25);
$("zoomFit").onclick=fit;
$("check").onclick=enterReview;$("back").onclick=back;
$("download").onclick=download;$("copy").onclick=copy;
function choose(){$("pick").click()}
$("chooseTop").onclick=choose;$("chooseEmpty").onclick=choose;
$("pick").onchange=()=>{
 const file=$("pick").files?.[0];if(file)loadFile(file);$("pick").value="";
};
$("loadDemo").onclick=async()=>{
 try{
  const r=await fetch("../docs/research/fixtures/T01_CHAT.png");
  if(!r.ok)throw Error("测试素材不可用");
  await loadFile(new File([await r.blob()],"T01_CHAT.png",{type:"image/png"}));
 }catch(e){status("合成示例无法加载："+e.message)}
};
$("dropzone").ondragover=e=>e.preventDefault();
$("dropzone").ondrop=e=>{
 e.preventDefault();const file=e.dataTransfer?.files?.[0];if(file)loadFile(file);
};
document.addEventListener("paste",e=>{
 if(e.target.closest("input,textarea,[contenteditable]"))return;
 const item=Array.from(e.clipboardData?.items||[]).find(x=>x.type.startsWith("image/"));
 if(!item)return;e.preventDefault();
 const file=item.getAsFile();if(file)loadFile(file);
});
document.addEventListener("keydown",e=>{
 if(e.target.closest("input,textarea,[contenteditable]"))return;
 const k=e.key.toLowerCase();
 if(e.key==="Escape"&&stage==="review"){back();return}
 if((e.ctrlKey||e.metaKey)&&k==="z"){e.preventDefault();if(e.shiftKey)redoAction();else undoAction();return}
 if((e.ctrlKey||e.metaKey)&&k==="y"){e.preventDefault();redoAction();return}
 if(stage!=="edit")return;
 if(k==="r")setMode("mask");
 if(k==="v")setMode("select");
 if(k==="h")setMode("pan");
 if(k==="0")fit();
 if(e.key==="Delete"||e.key==="Backspace")deleteSelected();
});
try{
 await init();ready=true;
 status("Rust/WASM 已加载，请选择已冻结的合成图片。");
}catch(e){status("Rust/WASM 初始化失败："+String(e?.message||e))}
setMode("mask");render();