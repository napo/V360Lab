# Changelog

All notable changes to V360Lab. The release workflow publishes the section
of the tagged version as the GitHub release notes.

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
