# App Store Connect helpers

Used to fill and submit Tracet's Mac App Store listing (2026-11-26). Run with
`signing.env` exported, but with the **App Manager** key: the upload key in
`signing.env` is refused for metadata.

    set -a; source signing.env; set +a
    export APPLE_API_KEY=C98NKWGURV APPLE_API_KEY_PATH=~/.appstoreconnect/private_keys/AuthKey_C98NKWGURV.p8
    python3 scripts/asc/asc.py GET '/v1/apps?filter[bundleId]=ca.polivoxia.tracet'
    python3 scripts/asc/meta.py                 # listing text, categories, age rating, price, availability
    python3 scripts/asc/shots.py <dir of 2880x1800 pngs>   # replaces the screenshot set

`meta.py` holds the IDs of the 0.2.1 version and its localization; a new
version gets new IDs (look them up with `asc.py`). Careful: re-running `meta.py` re-creates the price schedule and
availability, making China mainland available again (it was removed after
the first run because it needs an ICP filing). Not in the API: App
Privacy, the review contact, and the App Compliance forms.
