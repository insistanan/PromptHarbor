export type SkillProvider = 'claude' | 'codex';

export type SkillSourceKind = 'user' | 'project' | 'system';

export type SkillListItem = {
  id: string;
  provider: SkillProvider;
  providerLabel: string;
  sourceKind: SkillSourceKind;
  sourceLabel: string;
  localStatus: string;
  name: string;
  description: string;
  translatedName: string | null;
  translatedDescription: string | null;
  translatedAt: string | null;
  translatedProviderName: string | null;
  skillDir: string;
  skillFile: string;
  relativePath: string;
  contentHash: string;
};

export type SkillDetail = {
  skillFile: string;
  contentMd: string;
};

export type SkillTranslationResult = {
  translatedName: string;
  translatedDescription: string;
  providerId: string;
  providerName: string;
  model: string;
  updatedAt: string;
  cached: boolean;
};

export type InstalledSkillTarget = {
  provider: SkillProvider;
  providerLabel: string;
  targetDir: string;
  installedAt: string;
};

export type ImportedSkillPackageSummary = {
  packageId: string;
  importedAt: string;
  originalFileName: string;
  savedZipPath: string;
  stagedSkillDir: string;
  stagedSkillFile: string;
  skillDirName: string;
  name: string;
  description: string;
  installedTargets: InstalledSkillTarget[];
};

export type SkillInstallResult = {
  installed: boolean;
  requiresConfirmation: boolean;
  message: string;
  targets: InstalledSkillTarget[];
  conflicts: InstalledSkillTarget[];
};

export type SkillTransferResult = {
  transferred: boolean;
  requiresConfirmation: boolean;
  message: string;
  target: InstalledSkillTarget | null;
  conflicts: InstalledSkillTarget[];
};

export type SkillDeleteResult = {
  deleted: boolean;
  message: string;
  targetDir: string;
};

export type ImportedSkillPackageDeleteResult = {
  deleted: boolean;
  message: string;
  packageDir: string;
};
