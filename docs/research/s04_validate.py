#!/usr/bin/env python3
"""Audit S04 desk research and guard against treating draft slots as real participants.
Stdlib-only governance validator. It does NOT review legal applicability or recruit anyone.
"""
from collections import Counter
import csv
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
def validate(data, slots):
    assert data["schema"] == 1
    assert data["stage"] == "S04"
    assert data["document_review"] == "SAME_AGENT_COMPLETE"
    assert data["external_human_review"] == "PENDING"
    assert data["recruitment_authorized"] is False, "LIVE_RECRUITMENT_FALSIFIED"
    assert data["recruitment_status"] == "BLOCKED_BY_RECRUITMENT"
    assert all(x == 0 for x in data["metrics"].values()), "REAL_PARTICIPANT_NUMBERS_FALSIFIED"
    assert len(data["blockers"]) >= 6
    assert len(slots) == 10
    assert Counter(row["tool_order"] for row in slots) == Counter({"AB": 5, "BA": 5})
    assert all(row["slot_state"] == "UNASSIGNED" for row in slots)
    assert all(row["participant_id"] == "" for row in slots)
    assert all(row["consent_status"] == "NOT_REQUESTED" for row in slots)
    assert all(row["session_status"] == "NOT_STARTED" for row in slots)
    assert all(row["first_fixture"] != row["second_fixture"] for row in slots)
    expected = Counter({("Redacted", "T01_CHAT"):5, ("Redacted","T02_ORDER"):5,
                        ("Redaction Tools","T01_CHAT"):5, ("Redaction Tools","T02_ORDER"):5})
    observed = Counter()
    for row in slots:
        assert (row["tool_order"]=="AB")==(row["first_tool"]=="Redacted")
        for tool,fixture in [(row["first_tool"],row["first_fixture"]),
                             (row["second_tool"],row["second_fixture"])]:
            observed[(tool,fixture)] += 1
    assert observed == expected, (observed, expected)
    assert data["slots"]["assigned"] == 0
    assert data["hypotheses"]["A1"] == "OPEN"
    assert data["product_Rust_code_authorized"] is False if "product_Rust_code_authorized" in data else True
    assert "RUST_PRODUCT_CODE" in data["prohibited"]
    return True

def main():
    readiness=json.loads((ROOT/"STAGE01_S04_READINESS.json").read_text(encoding="utf-8"))
    with (ROOT/"STAGE01_S04_ALLOCATION_SLOTS.csv").open(encoding="utf-8",newline="") as source:
        rows=list(csv.DictReader(source))
    validate(readiness,rows)
    protocol=(ROOT/"STAGE01_S04_RESEARCH_PROTOCOL.md").read_text(encoding="utf-8")
    consent=(ROOT/"STAGE01_S04_CONSENT_AND_SCREENERS.md").read_text(encoding="utf-8")
    review=(ROOT/"STAGE01_S04_REVIEW_AND_RECRUITMENT_FEASIBILITY_2026-10-15.md").read_text(encoding="utf-8")
    oldpack=(ROOT/"STAGE01_PARTICIPANT_STUDY_PACK.md").read_text(encoding="utf-8")
    for token in ("AB/BA","T01","T02","A1","0"):
        assert token in protocol
    for token in ("[RESEARCH_CONTROLLER]","[CONTACT_CHANNEL]","[RETENTION_AND_WITHDRAWAL_PROCESS]",
                  "[INCENTIVE_AND_BUDGET_DECISION]","NOT_PUBLISHED","STOP_RECRUIT"):
        assert token in consent, ("CONSENT_GAP",token)
    for index in range(1,16):
        assert f"R{index:02d}" in review, ("AUDIT_GAP",index)
    assert len(oldpack)>1000, "SOURCE_PACK_MISSING"
    assert "SAME-AGENT" in review and "EXTERNAL HUMAN REVIEW PENDING" in review
    # Negative-control tests: the guard must reject an invented participant and live permission.
    made_up={**readiness, "recruitment_authorized": True}
    try:
        validate(made_up,rows)
    except AssertionError: pass
    else: raise AssertionError("NEGATIVE_TEST_LIVE_AUTHORISATION_NOT_CAUGHT")
    invented=[dict(r) for r in rows]
    invented[0]["participant_id"]="FICTIONAL_PERSON"
    try:
        validate(readiness,invented)
    except AssertionError: pass
    else: raise AssertionError("NEGATIVE_TEST_FAKE_PARTICIPANT_NOT_CAUGHT")
    print("S04_INTERNAL_DOCUMENT_REVIEW_AND_ALLOCATION_VALIDATION_PASS")
    print("NEGATIVE_TESTS_PASS: invented participant and live recruitment false approval both rejected")
    print("S04_ONLY_DOCUMENTS_AUDITED; EXTERNAL_HUMAN_ETHICS_REVIEW_PENDING; RECRUITED=0; S05_RECRUITMENT_BLOCKED")

if __name__ == "__main__": main()
