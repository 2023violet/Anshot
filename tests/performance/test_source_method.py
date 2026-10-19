"""Handwritten method counterexamples, not measured window evidence."""
from pathlib import Path
import copy, sys, unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'native_oracle'))
from run_source_checks import source_jobs, source_shot


def example():
    rows = []
    for request in [1, 2, 3]:
        base = request * 1000
        phases = dict(trigger=0, trigger_return=50, worker_start=10, read_complete=20,
                      decoded=200, cpu_prepared=800, send_invoked=801, received=810, finished_observed=811)
        phases.update(dict(cancel=400, cancel_return=410, discarded_stale=820) if request == 2
                      else dict(install_return=820, accepted_observed=830))
        rows.extend(dict(type='source_event', request=request, name=n, ms=base+t) for n, t in phases.items())
        rows.extend(dict(type='frame', frame=request*10+i, fresh=True, discard=False, source_request=request,
                         source_busy=True, callback_begin_ms=base+t, begin_ms=base+t+1, end_ms=base+t+2,
                         callback_end_ms=base+t+3, ui_cpu_ms=1, controller_cpu_ms=1,
                         callback_to_app_return_cpu_ms=2) for i, t in enumerate([100, 300]))
    return rows


class SourceMethod(unittest.TestCase):
    def test_valid_method_fixture(self):
        jobs = source_jobs(example())
        self.assertEqual([j['request'] for j in jobs], [1, 2, 3])
        self.assertEqual([j['busy_distinct_frames'] for j in jobs], [2, 2, 2])

    def rejected(self, change):
        rows = copy.deepcopy(example()); change(rows)
        with self.assertRaises((ValueError, RuntimeError)):
            source_jobs(rows)

    def test_post_completion_frames_cannot_supply_coverage(self):
        self.rejected(lambda r: [e.update(callback_end_ms=2000) for e in r if e['type']=='frame' and e['source_request']==1])

    def test_duplicate_frame_cannot_supply_coverage(self):
        self.rejected(lambda r: r.append(copy.deepcopy(next(e for e in r if e['type']=='frame'))))

    def test_cancel_must_return_inside_actual_cpu_interval(self):
        self.rejected(lambda r: next(e for e in r if e.get('name')=='cancel_return').update(ms=2900))

    def test_cancelled_candidate_cannot_be_accepted(self):
        self.rejected(lambda r: r.append(dict(type='source_event', request=2, name='accepted_observed', ms=2840)))

    def test_missing_read_phase_is_rejected(self):
        self.rejected(lambda r: r.remove(next(e for e in r if e.get('name')=='read_complete')))

    def test_duplicate_phase_is_rejected(self):
        self.rejected(lambda r: r.append(copy.deepcopy(next(e for e in r if e.get('name')=='decoded'))))

    def test_nonfinite_time_is_rejected(self):
        self.rejected(lambda r: next(e for e in r if e.get('name')=='decoded').update(ms=float('nan')))

    def test_wrong_request_frames_cannot_supply_coverage(self):
        self.rejected(lambda r: [e.update(source_request=999) for e in r if e['type']=='frame'])

    def test_discarded_pass_cannot_supply_coverage(self):
        self.rejected(lambda r: [e.update(discard=True) for e in r if e['type']=='frame'])

    def test_acceptance_cannot_precede_install(self):
        self.rejected(lambda r: next(e for e in r if e.get('name')=='accepted_observed').update(ms=1700))

    def test_file_read_frames_cannot_supply_cpu_coverage(self):
        def change(rows):
            next(e for e in rows if e.get('name')=='read_complete').update(ms=1350)
            next(e for e in rows if e.get('name')=='decoded').update(ms=1400)
        self.rejected(change)

    def test_cancel_in_file_read_does_not_prove_cpu_cancel(self):
        def change(rows):
            next(e for e in rows if e.get('name')=='read_complete' and e['request']==2).update(ms=2450)
            next(e for e in rows if e.get('name')=='decoded' and e['request']==2).update(ms=2500)
        self.rejected(change)

    def test_busy_edit_view_preserves_reviewed_old_png(self):
        job=source_jobs(example())[0]
        req=dict(name='busy',request=1,ms=1100,source_busy=True,reviewed=True,review_view=False,document=1,revision=1)
        shot=dict(reviewed=True,review_view=False,document=1,revision=1)
        source_shot(req,shot,job)

    def test_gpu_receipt_must_match_requested_document_and_view(self):
        job=source_jobs(example())[0]
        next(e for e in job['events'] if e['name']=='accepted_observed').update(document=2)
        req=dict(name='first',request=1,ms=1900,source_busy=False,reviewed=True,review_view=True,document=2,revision=1)
        for field, value in [('document',999),('revision',999),('review_view',False),('reviewed',False)]:
            shot=dict(reviewed=True,review_view=True,document=2,revision=1);shot[field]=value
            with self.subTest(field=field), self.assertRaises(RuntimeError):
                source_shot(req,shot,job)


if __name__ == '__main__':
    unittest.main()
