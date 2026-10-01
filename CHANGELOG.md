# Changelog

All notable changes to V360Lab. The release workflow publishes the section
of the tagged version as the GitHub release notes.

## Unreleased

- **Horizon levelling (experimental)**: the FIT accelerometer is decoded (with the camera's calibration) and used to straighten 360° images: a "Level horizon" switch in the 360° video view and in the frame extraction, and the camera's roll and pitch in the telemetry panel. Which sensor axis points forward still has to be confirmed on a real camera (see the README).
- **Tilt chart**: the camera's roll and pitch over the video, under the speed and altitude chart (tap to seek). Useful to check the levelling.
- **GPS jumps removed**: positions implying an impossible speed (well above the speed the GPS reports) are dropped from the track, distance, exports and frames; the panel shows how many.
- **Connected accessories**: a headset, Bluetooth sensor or ANT+ sensor connected to the camera (`btHeadset`, `btSensor`, `antSensor` in its status) is shown on the capture screen and in the status panel.
- The FIT decoder reads array fields (several samples per message).

## 0.4.0

- **Play videos** from the media library, streamed from the camera through the backend with seeking: the low-resolution copy (`.GLV`) by default, the original on request. 360° videos and photos open in the interactive 360° view.
- **Telemetry under the video**: the FIT file is decoded (native parser) and shown as a GPS track, speed and altitude charts and summary figures (distance, top and average speed, altitude range, climb), synchronized with playback. Tapping the track or the chart seeks the video.
- **Motion control** for the 360° view on phones: look around by moving the phone.
- **Favourites**: mark media with a star (`setFavorite`) and filter the library by favourites.
- **Sensors and media folders** under Advanced → Device (`sensors`, `mediaDirList`), and a **standby** button (`standby`).
- **Export the GPS track** of a video as GPX or GeoJSON, next to its downloads. The GeoJSON also gives, for each point, its UTC time and its position in the video.
- **Georeferenced frames**: extract frames from a video every N metres or seconds, as JPEG files with EXIF GPS and, for 360° videos, 2:1 equirectangular images with GPano panorama tags (Mapillary, Panoramax, photogrammetry), plus a `frames.geojson` index.
- **360° live preview fixed**: the VIRB squeezes the 360° preview into 16:9, which the viewer used to reject (it expected 2:1), so it stayed flat.
- **Author and source code**: the app (Advanced → About), the website and the README name the author, Maurizio Napolitano, and give the address of the source code, as the AGPL-3.0 asks.
- README: removed an outdated limitation (the live preview and file deletion exist); phase 1 of the roadmap marked as done.

## 0.3.0

- **Interactive 360° preview**: with the 360° lens, the live preview shows a perspective view of the sphere, as Garmin's app does: drag to look around, pinch or scroll to zoom, "Look ahead" to recenter. The flat (equirectangular) image is one tap away. Not yet tested on a real camera.
- **Smoother live preview**: the video packets are now read by a dedicated task with a larger receive buffer, and the decoder tolerates a longer queue. Before, lost packets froze the image until the next keyframe, which made the 360° preview look stroboscopic. Losses are logged every 10 seconds. When the image is waiting for a keyframe, the app asks the camera for one at once (`enableIDR`).
- **Find the camera** (Advanced → Device): make the camera beep and blink until you find it (`locate` / `found`).
- **Supported commands**: the app reads the camera's `commandList`, shows it under Advanced → Device, and hides the Wi-Fi tab when the camera does not list `networks`.
- **Camera Wi-Fi** (Advanced → Wi-Fi): save a network on the camera, choosing it from the networks the camera sees or typing its name, remove saved networks, and make the camera join one, as in Garmin's VIRB app. Network names and passwords are checked before they are sent. Not yet tested on a real camera.

## 0.2.0

- **Photo shutter as a lens diaphragm**: the photo and interval shutter is a six-blade diaphragm, closed at rest, that opens onto the lens when a picture is taken; during an interval capture it stays open with a stop sign.
- **Recording halo**: while recording, the video shutter pulses with a red glow and an expanding ring (still with reduced motion).
- **Faster live preview after a lens or mode change**: the camera stops its preview stream when the lens changes; the app now restarts it after 2 seconds instead of showing a frozen image for about 6 seconds. Tested on a VIRB 360 with firmware 4.20 (360° and front lens).
- **Android**: the APK targets the stable Android 16 SDK (API 36); fixed the release build setup.
- **Website** in English and Italian, with screenshots in each language.

## 0.1.0

First public release.

- Camera search (last camera, the camera's Wi-Fi, local network scan) with step-by-step feedback.
- Capture screen designed for phones: live preview, photo/video switch, lens (360°, front, rear, RAW) and mode settings as touch chips, big shutter button, recording timer, battery and free space.
- Media library with thumbnails; download of media, FIT telemetry and camera metadata; verified deletion.
- All camera settings, with readable names in English and Italian.
- Builds for Android (signed APK), Windows and Linux.
