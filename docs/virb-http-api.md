# VIRB camera API (reverse-engineered)

This document lists every camera command used by the official Garmin VIRB
Android app, recovered from its native library (`lib/arm64-v8a/libvirb-lib.so`,
C++ code built from Garmin's `shared/libs/camera` sources). The library keeps
its internal symbols, so each request builder (`camera::SnapPictureCommand`,
`camera::ConfigureNetworkCommand`, ...) was disassembled and the JSON it builds
was read directly.

Confidence markers:

- **verified**: the JSON keys and values were read from the disassembly.
- **inferred**: the value types or meaning come from surrounding code or
  strings, not from the request builder itself.

## Transport

- Every command is an HTTP `POST` to `http://<camera-ip>/virb` with a JSON body
  (`camera::Camera_t::SendCommand` builds `"http://" + ip + "/virb"`).
- The body always has a `"command"` key. Arguments are either extra top-level
  keys or nested under `"args"`, depending on the command (see below).
- Default IP on the camera's own access point: `192.168.0.1`.
- A response with `"result": 0` is treated as a failure
  ("Camera returned a 0 result"). An empty or non-JSON body is also a failure.
- The app sends one command at a time per camera and refuses a new
  status/features poll while the previous one is still pending.
- Timeouts (`camera::scCommandTimeout` and per-command overrides):
  - default: **10 s**
  - `deviceInfo`: **5 s**
  - `status`, `exportService*`: **30 s**

## Command reference

### Device and state

| Command | Body | Notes |
|---|---|---|
| `deviceInfo` | `{"command":"deviceInfo"}` | Response parsed for `model`, `firmware`, `deviceId`, `partNumber`, `macAddress`. verified |
| `status` | `{"command":"status"}` | Polled. Keys read: `totalSpace`, `availableSpace`, `recordingTime`, `photoCount`, `batteryChargingState`, `batteryLevel`, `recordingTimeRemaining`, `photosRemaining`, `apiMin`, `apiMax`, `wifiSignalStrength`, `wifiMode`, `antSensor`, `btSensor`, `btHeadset`, `wifiSensor`, `lastMediaEventTime`, `gpsLatitude`, `gpsLongitude`, `gpsAccuracy`, `gpsLastTime`. verified |
| `features` | `{"command":"features"}` | Polled. verified |
| `updateFeature` | `{"command":"updateFeature","feature":<string>,"value":<string>}` | Boolean features are sent as `"0"` / `"1"` strings. verified |
| `sensors` | `{"command":"sensors"}` | Response: `sensors[]`, each with `name`, `found`, a type (`LOCAL` or `ANT`) and units (`RPM`, `BPM`, `Meters/Second`, `Degrees/Second`, ...). verified |
| `commandList` | `{"command":"commandList"}` | Response: `commandList[]` of `{"command": <name>}`. The app uses it to check which commands the firmware supports. verified |

### Capture

| Command | Body | Notes |
|---|---|---|
| `startRecording` | `{"command":"startRecording"}` | verified |
| `stopRecording` | `{"command":"stopRecording"}` | verified |
| `snapPicture` | `{"command":"snapPicture"}` | No arguments. The `double` passed to the builder is the request **timeout**, not a self-timer. Self-timer is a feature (`selfTimer`). verified |
| `stopStillRecording` | `{"command":"stopStillRecording"}` | Stops interval or time-lapse photo capture. verified |
| `livePreview` | `{"command":"livePreview","streamType":"rtp","maxResolutionVertical":"<n>","liveStreamActive":"0"\|"1"}` | Response carries the stream `url`. `maxResolutionVertical` comes from an `unsigned short` and is sent as a string. `liveStreamActive` comes from a `LiveStreamState_t` and probably means the phone is live-broadcasting (inferred). Keys verified |
| `enableIDR` | `{"command":"enableIDR"}` | Asks for a keyframe on the preview stream. verified |

### Media

| Command | Body | Notes |
|---|---|---|
| `mediaList` | `{"command":"mediaList"}` or `{"command":"mediaList","path":<dir>}` | `path` is optional. Items carry `fitURL`, `lowResVideoPath`, `groupId`, lens mode, etc. verified |
| `mediaDirList` | `{"command":"mediaDirList"}` | Response key `mediaDirs`. verified |
| `deleteFile` | `{"command":"deleteFile","files":[<string>, ...]}` | verified |
| `deleteFile` (what the official app sends) | – | `mediaview::MediaItemDeleteService_t::GroupDeviceCommands` adds one entry per file: each source file of a video (`RawMovie_t::SourceFiles()` → `DeviceItemId()`), each photo of a photo set, or the item itself. A FIT file is never added: the FIT link (`FitUuid`) is only used to download telemetry. The official app therefore leaves FIT files in `GMetrix/` too. verified |
| `deleteFileGroup` | – | Only listed in the command-type table used with `commandList`. The app has no builder for it. inferred |
| `setFavorite` | `{"command":"setFavorite","file":<string>,"favorite":"true"\|"false"}` | `favorite` is a string, not a JSON boolean. verified |

### Camera utilities

| Command | Body | Notes |
|---|---|---|
| `locate` | `{"command":"locate"}` | Makes the camera signal its position (inferred from the app's "locate camera" feature). verified body |
| `found` | `{"command":"found"}` | Stops the `locate` signal. verified |
| `standby` | `{"command":"standby"}` | verified |
| `restoreDefaults` | `{"command":"restoreDefaults","type":"proPhoto"\|"proVideo"}` | Only resets the Pro photo or Pro video settings. verified |
| `getErrorLogURL` | `{"command":"getErrorLogURL"}` | verified |
| `deleteErrorLog` | `{"command":"deleteErrorLog"}` | verified |

### Wi-Fi networks

All network commands share `"command":"networks"` and select the operation with
`"subCommand"`. verified

| subCommand | Body |
|---|---|
| `getApSSID` | `{"command":"networks","subCommand":"getApSSID"}` |
| `getConfiguredNetworks` | `{"command":"networks","subCommand":"getConfiguredNetworks"}` |
| `getScannedNetworks` | `{"command":"networks","subCommand":"getScannedNetworks"}` |
| `configureNetwork` | `{"command":"networks","subCommand":"configureNetwork","args":{"type":…,"securityType":…,"ssid":…,"password":…}}` |
| `connectNetwork` | `{"command":"networks","subCommand":"connectNetwork","args":{"ssid":…}}` |
| `removeNetwork` | `{"command":"networks","subCommand":"removeNetwork","args":{"ssid":…}}` |

Details recovered from `virb::wifi::WifiConfigurationController_t` and
`WifiDiscoveryService_t`:

- `type` is `"station"` for a network the camera joins as a client (the
  default) and `"accessPoint"` to change the camera's own network.
- `securityType` is one of `WPA2`, `WPA`, `WEP`, `Open`.
- Network lists are read from `subCommand.networks[]`, each with `ssid` and
  `securityType`; `getApSSID` is read from `subCommand.ssid`.
- The app flow is `configureNetwork`, then (after asking the user)
  `connectNetwork`; the phone must then join the same network, and the app
  waits for the camera there. The camera keeps a maximum number of saved
  networks (the value was not recovered).
- The UI rejects a `configureNetwork` without a security type, SSID or
  password, and warns about SSID length, WPA password length and characters,
  and WEP key length.

### Firmware update

| Body | Notes |
|---|---|
| `{"command":"sw_update","args":"status"}` | Response includes `sw_updater_status_info`. verified |
| `{"command":"sw_update","args":"list"}` | Response includes `sw_updater_list` and `upload_url`. verified |
| `{"command":"sw_update","args":"apply"}` | Installs the uploaded update. verified |

To upload the firmware, the app builds a URL from `http://`, the camera IP, the
path the camera returns and the file name `GUPDATE.GCD`
(`camera::Camera_t::UrlForFirmwareUpdate`). The exact way these parts are
joined was not checked. inferred

### Export service (in-camera stitching and export)

| Body | Notes |
|---|---|
| `{"command":"exportServiceStatus"}` | Response: `state`, `version`, `bundleConfigured`, `settingsConfigured`, `error`, `exportedVideoUrl`, `progressPercent`, `progressFrameTime`. States/errors include `exporting`, `stopped`, `settingsNotConfigured`, `bundleNotConfigured`, `insuffecientDiskSpace` (sic). verified |
| `{"command":"exportServiceControl","action":"start"}` | verified |
| `{"command":"exportServiceControl","action":"stop"}` | verified |
| `{"command":"exportServiceConfigureSettings","args":{"width":u32,"height":u32,"frameRateNumerator":u32,"frameRateDenominator":u32,"bitrate":double,"audioFormat":int}}` | verified |

## Outside the `/virb` endpoint

### Wake-on-WLAN

`camera::CameraManager_t::WakeCameraAtAddress` opens a UDP socket and sends a
magic packet to **port 57034** (`0xDECA`), either to a known camera or to the
multicast address `224.0.0.1` (`WakeAllCameras`). The packet starts with
`FF FF FF FF FF FF` followed by the MAC repeated, as in standard Wake-on-LAN.
The string `WAKEUPGARMINDEV!` is also used when the packet is built. The exact
payload layout was not checked. Only cameras reporting `supportsWowlan` are
woken this way.

### Camera identification

The Java side has Garmin MAC OUI ranges (`00:05:4f`, `10:4e:89`, `10:c6:fc`,
`14:8f:21`) that it uses to recognise cameras. The app remembers each camera's
`lastKnownIp` and `lastKnownSsid`.

### Garmin cloud (not the camera)

`http://silver.garmin.com` / `http://bronze.garmin.com` `/OBN/OBNServlet`
(form-urlencoded) are used to post camera error logs and to check for firmware
updates.

## Differences from V360Lab (0.2.0)

- `livePreview`: V360Lab sends `streamType: "rtp"` (required on firmware
  4.20) but not `maxResolutionVertical` and `liveStreamActive`.
- `networks`: implemented as described above, not yet tested on a camera.
- Not implemented yet: `sensors`, `commandList`, `enableIDR`, `mediaDirList`,
  `setFavorite`, `locate`/`found`, `restoreDefaults`, error log,
  `sw_update`, `exportService*`, Wake-on-WLAN.
