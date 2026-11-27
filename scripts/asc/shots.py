import sys, os, json, hashlib, urllib.request, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); from asc import call
VER_L = "5095f035-9901-4611-825e-e340f8eb61fa"
DIR = sys.argv[1]
sets = call("GET", f"/v1/appStoreVersionLocalizations/{VER_L}/appScreenshotSets")["data"]
s = next((x for x in sets if x["attributes"]["screenshotDisplayType"] == "APP_DESKTOP"), None)
if s is None:
    s = call("POST", "/v1/appScreenshotSets", {"data": {"type": "appScreenshotSets", "attributes": {"screenshotDisplayType": "APP_DESKTOP"},
        "relationships": {"appStoreVersionLocalization": {"data": {"type": "appStoreVersionLocalizations", "id": VER_L}}}}})["data"]
SET = s["id"]
for old in call("GET", f"/v1/appScreenshotSets/{SET}/appScreenshots")["data"]:
    call("DELETE", f"/v1/appScreenshots/{old['id']}")
for name in sorted(os.listdir(DIR)):
    if not name.endswith(".png"): continue
    data = open(os.path.join(DIR, name), "rb").read()
    r = call("POST", "/v1/appScreenshots", {"data": {"type": "appScreenshots", "attributes": {"fileName": name, "fileSize": len(data)},
        "relationships": {"appScreenshotSet": {"data": {"type": "appScreenshotSets", "id": SET}}}}})
    if "HTTP_ERROR" in r: print("FAIL reserve", name, r); continue
    sid = r["data"]["id"]
    for op in r["data"]["attributes"]["uploadOperations"]:
        chunk = data[op["offset"]:op["offset"] + op["length"]]
        req = urllib.request.Request(op["url"], data=chunk, method=op["method"])
        for h in op["requestHeaders"]: req.add_header(h["name"], h["value"])
        urllib.request.urlopen(req, timeout=300).read()
    r = call("PATCH", f"/v1/appScreenshots/{sid}", {"data": {"type": "appScreenshots", "id": sid,
        "attributes": {"uploaded": True, "sourceFileChecksum": hashlib.md5(data).hexdigest()}}})
    print("FAIL commit" if "HTTP_ERROR" in r else "uploaded", name)
time.sleep(15)
for x in call("GET", f"/v1/appScreenshotSets/{SET}/appScreenshots")["data"]:
    a = x["attributes"]; print(a["fileName"], a["assetDeliveryState"]["state"], a["assetDeliveryState"].get("errors"))
