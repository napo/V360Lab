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
} from "../types/camera";
import type { AppError } from "../types/errors";
import { isConnectionLoss, toAppError } from "../utils/errors";

export type ConnectionState =
  | { status: "disconnected" }
  | { status: "connecting"; address: string; mock: boolean }
  | { status: "connected"; kind: CameraKind; address: string }
  | { status: "error"; error: AppError; address: string; mock: boolean };

export interface CameraContextValue {
  connection: ConnectionState;
  deviceInfo: DeviceInfo | null;
  status: CameraStatus | null;
  statusError: AppError | null;
  statusUpdatedAt: Date | null;
  refreshing: boolean;
  pendingAction: CameraAction | null;
  actionError: AppError | null;
  connect: (address: string, mock: boolean) => Promise<void>;
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
  const [status, setStatus] = useState<CameraStatus | null>(null);
  const [statusError, setStatusError] = useState<AppError | null>(null);
  const [statusUpdatedAt, setStatusUpdatedAt] = useState<Date | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [pendingAction, setPendingAction] = useState<CameraAction | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);

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

  const refreshAll = useCallback(async () => {
    try {
      setDeviceInfo(await cameraService.deviceInfo());
    } catch (e) {
      setStatusError(toAppError(e));
    }
    await refreshStatus();
  }, [refreshStatus]);

  const connect = useCallback(
    async (address: string, mock: boolean) => {
      setConnection({ status: "connecting", address, mock });
      setActionError(null);
      try {
        const info = await cameraService.connect(address, mock);
        clearThumbnailCache();
        setDeviceInfo(info.deviceInfo);
        applyStatus(info.status);
        setConnection({ status: "connected", kind: info.kind, address: info.address });
        void reloadSettings();
      } catch (e) {
        setConnection({ status: "error", error: toAppError(e), address, mock });
      }
    },
    [applyStatus, reloadSettings],
  );

  const disconnect = useCallback(async () => {
    try {
      await cameraService.disconnect();
    } finally {
      setConnection({ status: "disconnected" });
      setDeviceInfo(null);
      setStatus(null);
      setStatusError(null);
      setStatusUpdatedAt(null);
      clearThumbnailCache();
    }
  }, []);

  const runAction = useCallback(
    async (action: CameraAction) => {
      setPendingAction(action);
      setActionError(null);
      try {
        await cameraService.runAction(action);
      } catch (e) {
        setActionError(toAppError(e));
      } finally {
        setPendingAction(null);
        await refreshStatus();
      }
    },
    [refreshStatus],
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

  const value = useMemo<CameraContextValue>(
    () => ({
      connection,
      deviceInfo,
      status,
      statusError,
      statusUpdatedAt,
      refreshing,
      pendingAction,
      actionError,
      connect,
      disconnect,
      refreshStatus,
      refreshAll,
      runAction,
    }),
    [
      connection,
      deviceInfo,
      status,
      statusError,
      statusUpdatedAt,
      refreshing,
      pendingAction,
      actionError,
      connect,
      disconnect,
      refreshStatus,
      refreshAll,
      runAction,
    ],
  );

  return <CameraContext.Provider value={value}>{children}</CameraContext.Provider>;
}
