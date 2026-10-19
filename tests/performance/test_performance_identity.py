import unittest
from run_component_baseline import source_map

class CompiledFixtureIdentity(unittest.TestCase):
    def test_every_frozen_fixture_referenced_by_the_app_is_bound(self):
        sources=source_map()
        for name in ['T01_CHAT.png','T02_ORDER.png','T03_SUPPORT.png','T04_EDGES.png','T05_JPEG_EXIF.jpg']:
            self.assertIn('docs/research/fixtures/'+name,sources)
        for name in ['synthetic-noise-9mp.jpg','synthetic-two-frame.png','synthetic-metadata.png','synthetic-split-idat.png']:
            self.assertIn('tests/performance/fixtures/'+name,sources)

if __name__=='__main__':unittest.main()
