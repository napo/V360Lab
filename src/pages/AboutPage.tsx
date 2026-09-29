import { Panel } from "../components/Panel";
import { useAsyncResource } from "../hooks/useAsyncResource";
import { settingsService } from "../services/settingsService";

export function AboutPage() {
  const info = useAsyncResource(settingsService.appInfo);

  return (
    <div className="page page-narrow">
      <h1>About V360Lab</h1>
      <Panel title="V360Lab">
        <p>
          Open-source toolkit for Garmin VIRB 360 video, telemetry and computer vision.
          V360Lab talks to the camera over its local HTTP API, downloads media and FIT telemetry,
          and prepares data for geospatial and computer-vision processing.
        </p>
        <p className="mono small">
          {info.data
            ? `Version ${info.data.version}${info.data.debugBuild ? " (debug build)" : ""}`
            : info.error
              ? "Version unavailable"
              : "…"}
        </p>
        <p>
          Licensed under the GNU Affero General Public License v3.0 (AGPL-3.0). Everything runs
          locally: no data is sent to external services.
        </p>
      </Panel>
      <Panel title="Roadmap">
        <ol className="hint-list">
          <li>Camera connection, controls, media browser, media and FIT download (current)</li>
          <li>FIT parsing, timeline, GPS track, synchronized video and map</li>
          <li>Frame extraction, OpenCV processing, YOLO detection, SAM segmentation</li>
          <li>Georeferenced detections, GeoJSON / GeoParquet export, MapLibre visualization</li>
          <li>Depth estimation, photogrammetry, SfM, point clouds, 3D reconstruction</li>
        </ol>
      </Panel>
      <Panel title="Disclaimer">
        <p className="small">
          V360Lab is an independent open-source project and is not affiliated with, sponsored by,
          or endorsed by Garmin. Garmin and VIRB are trademarks of Garmin Ltd. or its subsidiaries.
        </p>
      </Panel>
    </div>
  );
}
