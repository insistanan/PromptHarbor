export type AppStatus = {
  appName: string;
  displayName: string;
  version: string;
  hookProtocolVersion: string;
  localEndpoint: string;
  dataPolicy: string;
  promptboxHome: string | null;
  configPath: string | null;
  databasePath: string | null;
  spoolPath: string | null;
  logsDir: string | null;
  hookBinaryPath: string | null;
  recordingPaused: boolean;
  maybeClosedAfterHours: number;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: number;
  autostart: boolean;
  configReady: boolean;
  hookBinaryReady: boolean;
  hookBinaryMessage: string;
  databaseReady: boolean;
  databaseMessage: string;
  sessionCount: number;
  promptEventCount: number;
  collectorReady: boolean;
  collectorMessage: string;
  importedSpoolEvents: number;
  receivedPromptEvents: number;
  pausedPromptEvents: number;
  startupErrors: string[];
};

export type HookPathStatus = {
  label: string;
  path: string;
};

export type HookAdapterStatus = {
  provider: string;
  providerLabel: string;
  expectedCommand: string;
  installed: boolean;
  ready: boolean;
  readable: boolean;
  message: string;
  configPaths: HookPathStatus[];
  backupPaths: HookPathStatus[];
  codexHooksEnabled: boolean | null;
};

export type RuntimeConfigDraft = {
  localEndpoint: string;
  recordingPaused: boolean;
  maybeClosedAfterHours: string;
  retainRawHookEvents: boolean;
  rawHookEventsRetentionDays: string;
  autostart: boolean;
};
