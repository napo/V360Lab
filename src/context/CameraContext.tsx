import {
  createContext,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useSettings } from "../hooks/useSettings";
import { cameraService } from "../services/cameraService";
import { clearThumbnailCache } from "../services/thumbnailService";
import type {
  CameraAction,
  CameraKind,
  CameraStatus,
  DeviceInfo,
  FeatureList,
} from "../types/camera";
import type { AppError } from "../types/errors";
import { isConnectionLoss, toAppError } from "../utils/errors";
import { commandSupported } from "../utils/commands";
import { featureValue, findFeature } from "../utils/features";

export type ConnectionState =
  | { status: "disconnected" }
  | { status: "connecting"; address: string; mock: boolean }
  | { status: "connected"; kind: CameraKind; address: string }
  | { status: "error"; error: AppError; address: string; mock: boolean };

export interface CameraContextValue {
  connection: ConnectionState;
  deviceInfo: DeviceInfo | null;
  /** Commands the firmware reports (`commandList`); null when unknown. */
  supportedCommands: string[] | null;
  /** False only when the camera's command list leaves `command` out. */
  supports: (command: string) => boolean;
  status: CameraStatus | null;
  statusError: AppError | null;
  statusUpdatedAt: Date | null;
  /** Camera features (settings), loaded after connecting and kept in sync
   * by `updateFeature`. Shared by the dashboard and the features page. */
  features: FeatureList | null;
  featuresError: AppError | null;
  featuresLoading: boolean;
  /** Mode from the `shootingMode` feature, for firmware without `mode` in status. */
  shootingMode: string | null;
  /** Key of the feature currently being changed. */
  pendingFeature: string | null;
  featureError: AppError | null;
  loadFeatures: () => Promise<FeatureList | null>;
  updateFeature: (key: string, value: string) => Promise<void>;
  refreshing: boolean;
  pendingAction: CameraAction | null;
  actionError: AppError | null;
  /** A photo interval (time-lapse) capture was started from this app and
   * not stopped yet. Firmware 4.20 status may not report it. */
  intervalActive: boolean;
  /** Connects and reports progress under `activityId`; resolves to success. */
  connect: (address: string, mock: boolean, activityId: string) => Promise<boolean>;
  disconnect: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshAll: () => Promise<void>;
  runAction: (action: CameraAction) => Promise<void>;
}

export const CameraContext = createContext<CameraContextValue | null>(null);

/** Consecutive unreachable/timeout status failures before reporting a lost connection. */
const MAX_STATUS_FAILURES = 3;

export function CameraProvider({ children }: { children: ReactNode }) {
  const { view, reload: reloadSettings } = useSettings();
  const [connection, setConnection] = useState<ConnectionState>({ status: "disconnected" });
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [supportedCommands, setSupportedCommands] = useState<string[] | null>(null);
  const [status, setStatus] = useState<CameraStatus | null>(null);
  const [statusError, setStatusError] = useState<AppError | null>(null);
  const [statusUpdatedAt, setStatusUpdatedAt] = useState<Date | null>(null);
  const [features, setFeatures] = useState<FeatureList | null>(null);
  const [featuresError, setFeaturesError] = useState<AppError | null>(null);
  const [featuresLoading, setFeaturesLoading] = useState(false);
  const [pendingFeature, setPendingFeature] = useState<string | null>(null);
  const [featureError, setFeatureError] = useState<AppError | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [pendingAction, setPendingAction] = useState<CameraAction | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);
  const [intervalActive, setIntervalActive] = useState(false);

  const inFlight = useRef(false);
  const refreshQueued = useRef(false);
  const failures = useRef(0);

  const applyStatus = useCallback((next: CameraStatus | null) => {
    setStatus(next);
    setStatusUpdatedAt(next ? new Date() : null);
    setStatusError(null);
    failures.current = 0;
  }, []);

  const refreshStatus = useCallback(async () => {
    if (inFlight.current) {
      // Make sure a refresh requested during a poll still happens afterwards.
      refreshQueued.current = true;
      return;
    }
    inFlight.current = true;
    setRefreshing(true);
    try {
      applyStatus(await cameraService.status());
    } catch (e) {
      const error = toAppError(e);
      setStatusError(error);
      if (error.kind === "notConnected") {
        setConnection({ status: "disconnected" });
      } else if (isConnectionLoss(error)) {
        failures.current += 1;
        if (failures.current >= MAX_STATUS_FAILURES) {
          setConnection((current) =>
            current.status === "connected"
              ? { status: "error", error, address: current.address, mock: current.kind === "mock" }
              : current,
          );
        }
      }
    } finally {
      inFlight.current = false;
      setRefreshing(false);
      if (refreshQueued.current) {
        refreshQueued.current = false;
        void refreshStatus();
      }
    }
  }, [applyStatus]);

  // The features request is slow (~2 s), so it is not part of status polling.
  const loadFeatures = useCallback(async () => {
    setFeaturesLoading(true);
    try {
      const list = await cameraService.features();
      setFeatures(list);
      setFeaturesError(null);
      return list;
    } catch (e) {
      setFeaturesError(toAppError(e));
      return null;
    } finally {
      setFeaturesLoading(false);
    }
  }, []);

  const updateFeature = useCallback(
    async (key: string, value: string) => {
      setPendingFeature(key);
      setFeatureError(null);
      try {
        // The camera answers with the complete, updated feature list.
        setFeatures(await cameraService.updateFeature(key, value));
      } catch (e) {
        setFeatureError(toAppError(e));
        void loadFeatures();
      } finally {
        setPendingFeature(null);
        void refreshStatus();
      }
    },
    [loadFeatures, refreshStatus],
  );

  const loadSupportedCommands = useCallback(async () => {
    try {
      setSupportedCommands(await cameraService.supportedCommands());
    } catch {
      setSupportedCommands(null);
    }
  }, []);

  const refreshAll = useCallback(async () => {
    try {
      setDeviceInfo(await cameraService.deviceInfo());
    } catch (e) {
      setStatusError(toAppError(e));
    }
    await refreshStatus();
    void loadFeatures();
    void loadSupportedCommands();
  }, [refreshStatus, loadFeatures, loadSupportedCommands]);

  const connect = useCallback(
    async (address: string, mock: boolean, activityId: string) => {
      setConnection({ status: "connecting", address, mock });
      setActionError(null);
      try {
        const info = await cameraService.connect(address, mock, activityId);
        clearThumbnailCache();
        setDeviceInfo(info.deviceInfo);
        applyStatus(info.status);
        setConnection({ status: "connected", kind: info.kind, address: info.address });
        void reloadSettings();
        void loadFeatures();
        void loadSupportedCommands();
        return true;
      } catch (e) {
        setConnection({ status: "error", error: toAppError(e), address, mock });
        return false;
      }
    },
    [applyStatus, reloadSettings, loadFeatures, loadSupportedCommands],
  );

  const disconnect = useCallback(async () => {
    try {
      await cameraService.disconnect();
    } finally {
      setConnection({ status: "disconnected" });
      setDeviceInfo(null);
      setSupportedCommands(null);
      setStatus(null);
      setStatusError(null);
      setStatusUpdatedAt(null);
      setFeatures(null);
      setFeaturesError(null);
      setIntervalActive(false);
      setFeatureError(null);
      clearThumbnailCache();
    }
  }, []);

  const runAction = useCallback(
    async (action: CameraAction) => {
      setPendingAction(action);
      setActionError(null);
      const photoInterval =
        featureValue(findFeature(features, "shootingMode")) === "photoShootingMode" &&
        featureValue(findFeature(features, "photoMode")) === "Timelapse";
      try {
        await cameraService.runAction(action);
        if (action === "snapPicture" && photoInterval) setIntervalActive(true);
        if (action === "stopStillRecording") setIntervalActive(false);
      } catch (e) {
        setActionError(toAppError(e));
      } finally {
        setPendingAction(null);
        await refreshStatus();
      }
    },
    [features, refreshStatus],
  );

  // Restore a connection kept by the backend (e.g. after a frontend reload).
  useEffect(() => {
    let cancelled = false;
    cameraService
      .activeConnection()
      .then((active) => {
        if (cancelled || !active) return;
        setConnection({ status: "connected", kind: active.kind, address: active.address });
        void refreshAll();
      })
      .catch(() => {
        /* Backend unavailable: stay disconnected. */
      });
    return () => {
      cancelled = true;
    };
  }, [refreshAll]);

  // Background status refresh, independent of the current page.
  const pollSecs = view?.settings.statusPollIntervalSecs ?? 5;
  useEffect(() => {
    if (connection.status !== "connected") return;
    const id = window.setInterval(() => void refreshStatus(), pollSecs * 1000);
    return () => window.clearInterval(id);
  }, [connection.status, pollSecs, refreshStatus]);

  const shootingMode = featureValue(findFeature(features, "shootingMode"));
  const supports = useCallback(
    (command: string) => commandSupported(supportedCommands, command),
    [supportedCommands],
  );

  const value = useMemo<CameraContextValue>(
    () => ({
      connection,
      deviceInfo,
      supportedCommands,
      supports,
      status,
      statusError,
      statusUpdatedAt,
      features,
      featuresError,
      featuresLoading,
      shootingMode,
      pendingFeature,
      featureError,
      loadFeatures,
      updateFeature,
      refreshing,
      pendingAction,
      actionError,
      intervalActive,
      connect,
      disconnect,
      refreshStatus,
      refreshAll,
      runAction,
    }),
    [
      connection,
      deviceInfo,
      supportedCommands,
      supports,
      status,
      statusError,
      statusUpdatedAt,
      features,
      featuresError,
      featuresLoading,
      shootingMode,
      pendingFeature,
      featureError,
      loadFeatures,
      updateFeature,
      refreshing,
      pendingAction,
      actionError,
      intervalActive,
      connect,
      disconnect,
      refreshStatus,
      refreshAll,
      runAction,
    ],
  );

  return <CameraContext.Provider value={value}>{children}</CameraContext.Provider>;
}
