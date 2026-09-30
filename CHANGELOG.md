# Changelog

All notable changes to V360Lab. The release workflow publishes the section
of the tagged version as the GitHub release notes.

## Unreleased

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
