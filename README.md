# V360Lab

Open-source toolkit for Garmin VIRB 360 video, telemetry and computer vision.

> V360Lab is an independent open-source project and is not affiliated with, sponsored by, or endorsed by Garmin. Garmin and VIRB are trademarks of Garmin Ltd. or its subsidiaries.

## Purpose

The Garmin VIRB 360 records spherical video together with rich sensor telemetry (GNSS, speed, altitude, IMU) stored as FIT files. The camera is discontinued, and its official software depends on desktop apps and services that are no longer maintained.

V360Lab is a desktop toolkit that:

- talks to the camera **directly over the local network**, using the VIRB HTTP API, with no Garmin cloud service involved;
- downloads media and the associated FIT telemetry into a predictable folder structure;
- prepares the data for later geospatial and computer-vision processing (see the [roadmap](#roadmap)).

## Current MVP capabilities

- Connect to a VIRB 360 by IP address or host name, check that it is reachable, and remember the last address that worked
- Read camera information: model, firmware, device ID, part number
- Read camera status: recording state, mode, battery, storage, GPS position; refreshed in the background
- Camera controls: start recording, stop recording, take photo. Controls that conflict with the current camera state are disabled.
- Browse the camera's reported features and settings (read-only), with a raw JSON view
- Media library: file name, type, date/time, duration, size, lens mode, thumbnail, preview URLs, FIT availability
- Download media, FIT telemetry and thumbnails, plus a `metadata.json` holding the original camera metadata
- Mock camera mode for development without hardware
- Error messages written for the user, with technical details available in debug mode

## Architecture

```
React UI (TypeScript)
   │  invoke() / events              src/services/*
   ▼
Tauri commands                        src-tauri/src/commands.rs
   │
   ▼
CameraClient trait                    src-tauri/src/camera/
   ├── GarminVirb360Client            src-tauri/src/virb/client.rs
   │      │  HTTP (reqwest, async)
   │      ▼
   │   Garmin VIRB 360 HTTP API       POST http://<camera>/virb
   └── MockVirb360Client              src-tauri/src/virb/mock.rs
```

The UI never talks to the camera directly. Every request, including thumbnails, goes through Rust. This avoids CORS and webview network restrictions and keeps device communication out of the UI.

### Backend (`src-tauri/src`)

| Module | Responsibility |
|---|---|
| `camera/` | Camera-agnostic `CameraClient` trait, domain models (`DeviceInfo`, `CameraStatus`, `FeatureList`, `MediaItem`), typed `CameraError`, address validation |
| `virb/commands.rs` | VIRB API command names and payloads |
| `virb/models.rs` | Tolerant parsing of VIRB JSON into domain models; original JSON is preserved in `raw` |
| `virb/errors.rs` | Maps transport/HTTP/protocol failures to typed errors |
| `virb/client.rs` | `GarminVirb360Client`: timeouts, logging, streamed downloads |
| `virb/mock.rs` | `MockVirb360Client`: realistic simulated camera |
| `downloads/` | Folder layout, file-name sanitization, media/FIT/thumbnail download, `metadata.json` |
| `telemetry/` | FIT header validation; extension point for a future FIT decoder |
| `settings.rs` | Persisted user settings (JSON in the app config directory) |
| `commands.rs` | Tauri commands exposed to the UI |
| `error.rs` | `AppError`, serialized to the UI as `{ kind, message, detail }` |

To support another 360 camera, implement `CameraClient` for it. Commands, downloads and the UI do not change.

### Frontend (`src`)

| Folder | Content |
|---|---|
| `services/` | Typed wrappers around Tauri commands (the only place that calls `invoke`) |
| `context/` | Settings, camera connection and download state (React context, no Redux) |
| `hooks/` | `useCamera`, `useSettings`, `useDownloads`, `useAsyncResource`, `useThumbnail` |
| `pages/` | Connection, Dashboard, Media, Camera Features, Settings, About |
| `components/` | Reusable UI parts, grouped by feature |
| `types/` | TypeScript mirrors of the Rust models |
| `utils/` | Formatting and error helpers |

The connection is modelled explicitly as `disconnected | connecting | connected | error`. Camera status is polled from the app root, so it keeps updating whichever page is open.

### Downloaded file layout

```
<download directory>/            default: ~/Downloads/V360Lab
  2024-07-03/                    capture date (UTC)
    V0010042/                    recording name
      V0010042.MP4               original media file
      2024-07-03-09-46-40.fit    FIT telemetry, when available
      thumbnail.jpg              when available
      metadata.json              original camera metadata + download record
```

`metadata.json` contains `cameraMetadata` (the media entry exactly as the camera returned it), a `normalized` view, the list of downloaded files and any warnings. Files are written as `*.part` and renamed only when complete. A media file that already exists with the expected size is kept and not downloaded again.

## Development setup

Requirements:

- [Node.js](https://nodejs.org/) 20 or later and npm
- [Rust](https://rustup.rs/) 1.82 or later
- Tauri 2 system dependencies for your platform: see <https://v2.tauri.app/start/prerequisites/>
  (on Debian/Ubuntu: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`)

Install the JavaScript dependencies (the Tauri CLI is included as a dev dependency):

```bash
npm install
```

## Running in development mode

```bash
npm run tauri dev
```

This starts Vite on `http://localhost:1420` and opens the desktop window with hot reload. Backend logs go to the terminal. Debug builds log every camera request and response at debug level; response bodies are truncated and binary content is never logged. Use `RUST_LOG` to change verbosity, e.g. `RUST_LOG=v360lab_lib=trace`.

**Without a camera:** tick *Use simulated camera* on the connection screen, or enable *Mock mode* in Settings. The mock returns realistic device information, status, features and media entries. Recording, photos and downloads (placeholder files and valid empty FIT files) all work.

Running `npm run dev` alone serves the UI in a normal browser, but without the Rust backend. The app then reports that the backend is unavailable.

### Tests

```bash
# Rust: response parsing, malformed responses, HTTP error handling (mocked
# with wiremock), downloads, settings, mock camera. No hardware needed.
cd src-tauri && cargo test

# Frontend: formatting and error helpers
npm test

# Type checking
npm run typecheck
```

## Building

```bash
npm run tauri build
```

Installers and bundles are written to `src-tauri/target/release/bundle/`.

## Camera connection requirements

- Enable Wi-Fi on the VIRB 360 and connect this computer to the camera's network. Alternatively, put both on the same local network.
- When the camera acts as an access point it is typically reachable at `192.168.0.1`. This is only a default; enter the actual address when it differs.
- The address field accepts `192.168.0.1`, `192.168.0.1:8080` or `http://virb.local`. Paths, credentials and non-HTTP schemes are rejected.
- V360Lab contacts only the configured address. URLs reported by the camera (media, thumbnails, FIT) are re-anchored onto that address, redirects are not followed and system proxies are bypassed. No data is sent to external services.

### Timeouts

| Operation | Timeout |
|---|---|
| TCP connect | 3 s |
| API command / thumbnail | 10 s total |
| Download | 30 s without data (no total limit, since videos can be several GB) |

## VIRB API assumptions

The client follows the publicly known VIRB network API, but has not been validated against every firmware. Assumptions:

- Commands are `POST /virb` with body `{"command": "<name>"}`. Responses are JSON objects with `"result": 1` on success and `0` on failure.
- `deviceInfo` returns `{"deviceInfo": [{model, firmware, deviceId, partNumber, type, ...}]}`.
- `status` returns flat fields such as `state` (`idle` / `recording`), `mode`, `batteryLevel` (percent), `batteryChargingState`, `totalSpace` / `availableSpace` (**assumed to be bytes**), `recordingTime`, `recordingTimeRemaining`, `gpsLatitude`, `gpsLongitude`.
- `features` returns `{"features": [{feature, description, type, value, options, optionSummary, enabled}]}`.
- `mediaList` returns `{"media": [{name, type, date, duration, fileSize, lensMode, url, thumbUrl, lowResVideoPath, fitURL, ...}]}`. `date` is **assumed to be Unix seconds (UTC)**. An empty `fitURL` means no telemetry.
- HTTP 404/405/501, or an error message mentioning an unknown/unsupported command, is reported as "unsupported command".

Parsing is deliberately tolerant. Every field is optional, numbers may arrive as strings, several field-name aliases are accepted, and unknown properties are kept in `raw` so that firmware differences can be inspected in debug mode.

## Known limitations

- **Not yet verified against a physical VIRB 360.** See the checklist below.
- Camera settings are read-only; `updateFeature` is not implemented.
- FIT files are downloaded and their header is validated, but not decoded.
- No live preview (RTSP) and no deletion of files on the camera.
- The capture date folder uses UTC.
- Only one camera can be connected at a time.

### To verify with a real camera

1. The command endpoint path (`/virb`) and the `result` convention on current VIRB 360 firmware.
2. Units of `totalSpace` / `availableSpace` (bytes vs. KB/MB) and of `batteryLevel`.
3. The `state` values reported while recording, and whether `mode` is a string or a number.
4. The `date` format in `mediaList` (Unix seconds, milliseconds or another epoch) and its time zone.
5. The exact field names for thumbnails, low-resolution previews and FIT files (`thumbUrl`, `lowResVideoPath`, `fitURL`).
6. How raw (unstitched) dual-lens recordings appear in the media list.
7. Whether `snapPicture` is accepted while recording or in video mode.
8. The response to an unknown command.
9. Whether media URLs contain the camera's own IP (they are re-anchored to the configured address either way).

Please report findings (with the raw JSON from debug mode) in an issue.

## Roadmap

**Phase 1: camera toolkit (current)**
- VIRB connection
- Status
- Camera controls
- Media browser
- Media/FIT download

**Phase 2: telemetry**
- FIT parsing (Garmin FIT SDK or a compatible parser; see `src-tauri/src/telemetry/mod.rs`)
- Timeline
- GPS track
- Synchronized video + map

**Phase 3: computer vision**
- Frame extraction
- OpenCV processing
- Object detection
- YOLO
- SAM segmentation

**Phase 4: geospatial outputs**
- Georeferenced detections
- GeoJSON / GeoParquet export
- MapLibre visualization

**Phase 5: 3D**
- Depth estimation
- Photogrammetry
- SfM
- Point clouds
- Experimental 3D reconstruction

## License

V360Lab is licensed under the [GNU Affero General Public License v3.0](LICENSE) (AGPL-3.0).

## Disclaimer

V360Lab is an independent open-source project and is not affiliated with, sponsored by, or endorsed by Garmin. Garmin and VIRB are trademarks of Garmin Ltd. or its subsidiaries.
