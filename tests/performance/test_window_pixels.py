import tempfile,unittest
from pathlib import Path
from PIL import Image,ImageDraw
from run_window_checks import native_pixels

class WindowPixelChecks(unittest.TestCase):
    def setUp(self):
        self.directory=tempfile.TemporaryDirectory();self.addCleanup(self.directory.cleanup)
        self.source=Path(self.directory.name)/'source.png';self.shot=Path(self.directory.name)/'shot.png'
        self.image=Image.new('RGBA',(900,700),(129,3,1,255));self.image.save(self.source)
        self.metric={'zoom':1,'rect':[0,0,900,700],'clip':[0,0,900,700]}

    def test_one_pixel_line_error_between_old_seven_pixel_samples_is_rejected(self):
        image=self.image.copy();ImageDraw.Draw(image).line((0,350,899,350),fill=(0,0,0,255));image.save(self.shot)
        with self.assertRaises(AssertionError):native_pixels(self.source,self.shot,self.metric,1,[],False)

    def test_review_without_any_visible_cover_is_rejected(self):
        self.image.save(self.shot)
        with self.assertRaises(AssertionError):native_pixels(self.source,self.shot,self.metric,1,[[1000,1000,1100,1100]],True)

    def test_required_review_cannot_be_satisfied_by_edit_state(self):
        self.image.save(self.shot)
        with self.assertRaises(AssertionError):native_pixels(self.source,self.shot,self.metric,1,[[200,200,300,300]],False,require_cover=True)

    def test_correct_review_checks_cover_and_uncovered_pixels(self):
        image=self.image.copy();ImageDraw.Draw(image).rectangle((200,200,299,299),fill=(17,24,39,255));image.save(self.shot)
        result=native_pixels(self.source,self.shot,self.metric,1,[[200,200,300,300]],True)
        self.assertGreater(result['covered_samples'],500)
        self.assertGreater(result['uncovered_samples'],500)

if __name__=='__main__':unittest.main()
