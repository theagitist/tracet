import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from asc import call
APP="6839759747"; INFO="e24a74bd-b8e2-4276-b760-523a6a67a8e8"
INFO_L="796424b4-6dd7-4254-8c57-fede57fe1775"
VER="b5c1482a-fd9d-401f-b13f-a6ef4a3e2a51"; VER_L="5095f035-9901-4611-825e-e340f8eb61fa"

def ok(label, r):
    if "HTTP_ERROR" in r:
        print("FAIL", label, json.dumps(r["body"].get("errors", r["body"]))[:800])
    else:
        print("ok  ", label)
    return r

DESC = """Tracet turns recordings into transcripts you can trust, without sending a second of audio to the cloud.

Open an audio or video file and Tracet transcribes it with whisperX, aligns every word to the audio, and works out who is speaking. Everything runs on your Mac: no account, no subscription, no upload.

WHAT YOU GET
- Accurate transcription with the Whisper large-v3-turbo model, word-level timing and alignment.
- Speaker identification: each voice gets its own label, and you can give it a name once for the whole transcript.
- Confidence highlighting: words the model is unsure of are tinted, so you know where to listen again.
- Per-line playback: click any line to hear exactly that part of the recording.
- Inline editing: fix any word, and your edits carry into every export.
- Projects: save the transcript together with its audio in one .tracet file and pick up later.
- Exports: plain text, Markdown, and an AI-ready Markdown with structured metadata and numbered segments.

PRIVATE BY DESIGN
Your recordings and transcripts never leave your Mac. Tracet has no analytics, no tracking and no servers of its own. The only time it goes online is to download the speech model once, the first time you transcribe.

FILES
Opens mp3, wav, flac, m4a, ogg, aac, mp4, mkv, avi, webm and mov files with common audio codecs.

REQUIREMENTS
A Mac with Apple Silicon (M1 or later). The first transcription downloads the speech model once (about 1.6 GB); after that Tracet works offline. Tracet shows which models your Mac's memory can run.

Tracet is open source (MIT): github.com/theagitist/tracet"""

NOTES = """Tracet transcribes local audio and video files on device. No account or sign-in is needed.

To test: click Open Media, choose any short recording with speech (a voice memo or a video with two people talking works well), and wait for the transcript. Speakers are labelled SPEAKER_00, SPEAKER_01 and can be renamed in the right-hand panel. Click the play button on any line to hear it. Export is in the toolbar.

The first transcription downloads the Whisper speech model (about 1.6 GB) from huggingface.co, once, with a progress bar; it is model data (weights), not executable code, and it is stored in the app's container. Later transcriptions work offline. The speaker identification model ships inside the app.

All processing is local; Tracet sends no user data anywhere. The app is open source: https://github.com/theagitist/tracet"""

ok("app info: subtitle, privacy URL", call("PATCH", f"/v1/appInfoLocalizations/{INFO_L}", {"data": {"type": "appInfoLocalizations", "id": INFO_L, "attributes": {
    "subtitle": "Private transcription on Mac", "privacyPolicyUrl": "https://tracet.polivoxia.ca/privacy"}}}))
ok("categories", call("PATCH", f"/v1/appInfos/{INFO}", {"data": {"type": "appInfos", "id": INFO, "relationships": {
    "primaryCategory": {"data": {"type": "appCategories", "id": "PRODUCTIVITY"}},
    "secondaryCategory": {"data": {"type": "appCategories", "id": "UTILITIES"}}}}}))
enums=["alcoholTobaccoOrDrugUseOrReferences","contests","gamblingSimulated","gunsOrOtherWeapons","medicalOrTreatmentInformation","profanityOrCrudeHumor","sexualContentGraphicAndNudity","sexualContentOrNudity","horrorOrFearThemes","matureOrSuggestiveThemes","violenceCartoonOrFantasy","violenceRealisticProlongedGraphicOrSadistic","violenceRealistic"]
bools=["advertising","ageAssurance","datingAndRelationships","gambling","healthOrWellnessTopics","lootBox","messagingAndChat","parentalControls","sharedLocation","socialMedia","socialMediaAgeRestricted","unrestrictedWebAccess","userGeneratedContent"]
attrs = {k: "NONE" for k in enums}; attrs.update({k: False for k in bools})
r = ok("age rating", call("PATCH", f"/v1/ageRatingDeclarations/{INFO}", {"data": {"type": "ageRatingDeclarations", "id": INFO, "attributes": attrs}}))
ok("content rights", call("PATCH", f"/v1/apps/{APP}", {"data": {"type": "apps", "id": APP, "attributes": {"contentRightsDeclaration": "USES_THIRD_PARTY_CONTENT"}}}))
ok("version 0.2.1, copyright", call("PATCH", f"/v1/appStoreVersions/{VER}", {"data": {"type": "appStoreVersions", "id": VER, "attributes": {
    "versionString": "0.2.1", "copyright": "2026 Adrian Murillo"}}}))
ok("description, keywords, URLs", call("PATCH", f"/v1/appStoreVersionLocalizations/{VER_L}", {"data": {"type": "appStoreVersionLocalizations", "id": VER_L, "attributes": {
    "description": DESC,
    "keywords": "transcription,transcribe,speech to text,whisper,speakers,diarization,offline,private,interview",
    "promotionalText": "Tracet transcribes audio and video on your Mac, names who spoke, and highlights words it is unsure of. Nothing leaves your Mac.",
    "supportUrl": "https://tracet.polivoxia.ca/support", "marketingUrl": "https://tracet.polivoxia.ca"}}}))
rd = call("GET", f"/v1/appStoreVersions/{VER}/appStoreReviewDetail")
if rd.get("data"):
    rid = rd["data"]["id"]
    ok("review notes", call("PATCH", f"/v1/appStoreReviewDetails/{rid}", {"data": {"type": "appStoreReviewDetails", "id": rid, "attributes": {"notes": NOTES, "demoAccountRequired": False}}}))
else:
    ok("review notes (new)", call("POST", "/v1/appStoreReviewDetails", {"data": {"type": "appStoreReviewDetails", "attributes": {"notes": NOTES, "demoAccountRequired": False},
        "relationships": {"appStoreVersion": {"data": {"type": "appStoreVersions", "id": VER}}}}}))

# Price: free, base territory USA.
pp = call("GET", f"/v1/apps/{APP}/appPricePoints?filter[territory]=USA&limit=200")
free = [p["id"] for p in pp.get("data", []) if float(p["attributes"]["customerPrice"]) == 0]
if not free:
    print("FAIL price points", json.dumps(pp)[:400])
else:
    ok("price: free", call("POST", "/v1/appPriceSchedules", {
        "data": {"type": "appPriceSchedules", "relationships": {
            "app": {"data": {"type": "apps", "id": APP}},
            "baseTerritory": {"data": {"type": "territories", "id": "USA"}},
            "manualPrices": {"data": [{"type": "appPrices", "id": "${p0}"}]}}},
        "included": [{"type": "appPrices", "id": "${p0}", "attributes": {"startDate": None},
            "relationships": {"appPricePoint": {"data": {"type": "appPricePoints", "id": free[0]}}}}]}))

# Availability: every territory, plus new ones.
terr = call("GET", "/v1/territories?limit=200")["data"]
inc = [{"type": "territoryAvailabilities", "id": f"${{t{i}}}", "attributes": {"available": True},
        "relationships": {"territory": {"data": {"type": "territories", "id": t["id"]}}}} for i, t in enumerate(terr)]
ok(f"availability: {len(terr)} territories", call("POST", "/v2/appAvailabilities", {
    "data": {"type": "appAvailabilities", "attributes": {"availableInNewTerritories": True},
        "relationships": {"app": {"data": {"type": "apps", "id": APP}},
            "territoryAvailabilities": {"data": [{"type": "territoryAvailabilities", "id": x["id"]} for x in inc]}}},
    "included": inc}))
