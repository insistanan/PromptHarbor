import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type ReactNode,
} from 'react';
import {
  Archive,
  ArrowRightLeft,
  Bot,
  Boxes,
  CheckCircle2,
  FileArchive,
  FileText,
  FolderOpen,
  Languages,
  PackagePlus,
  RefreshCw,
  Shield,
  Sparkles,
  Trash2,
  Upload,
} from 'lucide-react';
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as api from '../../api';
import type {
  CustomProviderSummary,
  ImportedSkillPackageDeleteResult,
  ImportedSkillPackageSummary,
  SkillDeleteResult,
  SkillDetail,
  SkillInstallResult,
  SkillListItem,
  SkillProvider,
  SkillSourceKind,
  SkillTransferResult,
  SkillTranslationResult,
} from '../../appTypes';

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const providerOrder = ['claude', 'codex'] as const;
const sourceOrder: SkillSourceKind[] = ['user', 'project', 'system'];
const zipDropIdleMessage = '点击选择 zip 文件，或把单个 .zip 直接拖到当前窗口。';

export function SkillsWorkspace({
  onError,
  onNotice,
}: {
  onError: (message: string | null) => void;
  onNotice: (message: string) => void;
}) {
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [skills, setSkills] = useState<SkillListItem[]>([]);
  const [selectedProvider, setSelectedProvider] = useState<SkillProvider>('claude');
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null);
  const [detailCache, setDetailCache] = useState<Record<string, SkillDetail>>({});
  const [detailLoadingId, setDetailLoadingId] = useState<string | null>(null);

  const [translationProviders, setTranslationProviders] = useState<CustomProviderSummary[]>([]);
  const [providersLoading, setProvidersLoading] = useState(true);
  const [selectedTranslationProviderId, setSelectedTranslationProviderId] = useState('');
  const [translatingSkillId, setTranslatingSkillId] = useState<string | null>(null);

  const zipFileInputRef = useRef<HTMLInputElement | null>(null);
  const onErrorRef = useRef(onError);
  const importingZipRef = useRef(false);

  const [zipPathDraft, setZipPathDraft] = useState('');
  const [importingZip, setImportingZip] = useState(false);
  const [zipDropState, setZipDropState] = useState<'idle' | 'accept' | 'reject'>('idle');
  const [zipDropMessage, setZipDropMessage] = useState(zipDropIdleMessage);

  const [importedPackages, setImportedPackages] = useState<ImportedSkillPackageSummary[]>([]);
  const [packagesLoading, setPackagesLoading] = useState(true);
  const [selectedPackageId, setSelectedPackageId] = useState<string | null>(null);
  const [packageTargetSkillName, setPackageTargetSkillName] = useState('');
  const [packageInstallTargets, setPackageInstallTargets] = useState<{
    claude: boolean;
    codex: boolean;
  }>({ claude: true, codex: false });
  const [installingPackageId, setInstallingPackageId] = useState<string | null>(null);
  const [deletingPackageId, setDeletingPackageId] = useState<string | null>(null);

  const [exportingSkillId, setExportingSkillId] = useState<string | null>(null);
  const [deletingSkillId, setDeletingSkillId] = useState<string | null>(null);
  const [transferringSkillId, setTransferringSkillId] = useState<string | null>(null);
  const [transferTargetProvider, setTransferTargetProvider] = useState<SkillProvider>('codex');
  const [transferTargetSkillName, setTransferTargetSkillName] = useState('');

  const loadSkills = (
    mode: 'initial' | 'refresh' = 'initial',
    preferredProvider?: SkillProvider,
  ) => {
    if (mode === 'refresh') {
      setRefreshing(true);
    } else {
      setLoading(true);
    }

    api.listSkills<SkillListItem[]>()
      .then((nextSkills) => {
        setSkills(nextSkills);
        setDetailCache({});
        onError(null);

        const availableProviders = providerOrder.filter((provider) =>
          nextSkills.some((item) => item.provider === provider),
        );
        const providerCandidate = preferredProvider ?? selectedProvider;
        const nextProvider =
          availableProviders.find((provider) => provider === providerCandidate) ??
          availableProviders[0] ??
          providerOrder[0];
        setSelectedProvider(nextProvider);

        const nextProviderSkills = nextSkills.filter((item) => item.provider === nextProvider);
        setSelectedSkillId((currentId) => {
          if (currentId && nextProviderSkills.some((item) => item.id === currentId)) {
            return currentId;
          }
          return nextProviderSkills[0]?.id ?? null;
        });
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => {
        setLoading(false);
        setRefreshing(false);
      });
  };

  const loadProviders = () => {
    setProvidersLoading(true);
    api.listCustomProviders<CustomProviderSummary[]>()
      .then((providers) => {
        const nextProviders = providers.filter(
          (item) => item.enabled && item.supported && item.secretConfigured,
        );
        setTranslationProviders(nextProviders);
        setSelectedTranslationProviderId((current) => {
          if (current && nextProviders.some((item) => item.id === current)) {
            return current;
          }
          return nextProviders[0]?.id ?? '';
        });
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setProvidersLoading(false));
  };

  const loadImportedPackages = () => {
    setPackagesLoading(true);
    api.listImportedSkillPackages<ImportedSkillPackageSummary[]>()
      .then((packages) => {
        setImportedPackages(packages);
        setSelectedPackageId((current) => {
          if (current && packages.some((item) => item.packageId === current)) {
            return current;
          }
          return packages[0]?.packageId ?? null;
        });
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setPackagesLoading(false));
  };

  useEffect(() => {
    loadSkills();
    loadProviders();
    loadImportedPackages();
  }, []);

  useEffect(() => {
    onErrorRef.current = onError;
  }, [onError]);

  useEffect(() => {
    importingZipRef.current = importingZip;
  }, [importingZip]);

  const providerCounts = useMemo(
    () =>
      providerOrder.reduce<Record<SkillProvider, number>>((result, provider) => {
        result[provider] = skills.filter((item) => item.provider === provider).length;
        return result;
      }, { claude: 0, codex: 0 }),
    [skills],
  );

  const providerSkills = useMemo(
    () => skills.filter((item) => item.provider === selectedProvider),
    [selectedProvider, skills],
  );

  const groupedSkills = useMemo(
    () =>
      sourceOrder
        .map((sourceKind) => ({
          sourceKind,
          items: providerSkills.filter((item) => item.sourceKind === sourceKind),
        }))
        .filter((group) => group.items.length > 0),
    [providerSkills],
  );

  const selectedSkill =
    providerSkills.find((item) => item.id === selectedSkillId) ?? providerSkills[0] ?? null;
  const selectedSkillDetail = selectedSkill ? detailCache[selectedSkill.id] ?? null : null;
  const detailLoading = Boolean(selectedSkill && detailLoadingId === selectedSkill.id);

  const selectedPackage =
    importedPackages.find((item) => item.packageId === selectedPackageId) ?? importedPackages[0] ?? null;

  const providerUserSkillCount = providerSkills.filter((item) => item.sourceKind === 'user').length;
  const providerReadonlySkillCount = providerSkills.filter((item) => item.sourceKind !== 'user').length;
  const providerTranslatedCount = providerSkills.filter((item) => Boolean(item.translatedAt)).length;

  const canTranslate =
    Boolean(selectedSkill) &&
    Boolean(selectedTranslationProviderId) &&
    translatingSkillId !== selectedSkill?.id;
  const canInstallSelectedPackage =
    Boolean(selectedPackage) &&
    !installingPackageId &&
    (packageInstallTargets.claude || packageInstallTargets.codex);
  const canTransferSelectedSkill =
    Boolean(selectedSkill) &&
    selectedSkill.sourceKind === 'user' &&
    transferTargetProvider !== selectedSkill.provider &&
    transferTargetSkillName.trim().length > 0 &&
    !transferringSkillId;

  useEffect(() => {
    if (!providerSkills.length) {
      if (selectedSkillId !== null) {
        setSelectedSkillId(null);
      }
      return;
    }

    if (!selectedSkillId || !providerSkills.some((item) => item.id === selectedSkillId)) {
      setSelectedSkillId(providerSkills[0].id);
    }
  }, [providerSkills, selectedSkillId]);

  useEffect(() => {
    if (!selectedSkill || detailCache[selectedSkill.id]) {
      return;
    }

    setDetailLoadingId(selectedSkill.id);
    api.readSkillDetail<SkillDetail>({ skillFile: selectedSkill.skillFile })
      .then((detail) => {
        setDetailCache((current) => ({
          ...current,
          [selectedSkill.id]: detail,
        }));
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => {
        setDetailLoadingId((current) => (current === selectedSkill.id ? null : current));
      });
  }, [detailCache, onError, selectedSkill]);

  useEffect(() => {
    if (!selectedPackage) {
      setPackageTargetSkillName('');
      return;
    }
    setPackageTargetSkillName(selectedPackage.skillDirName);
  }, [selectedPackage?.packageId]);

  useEffect(() => {
    if (!selectedSkill) {
      setTransferTargetSkillName('');
      return;
    }

    const nextTarget = otherProvider(selectedSkill.provider);
    setTransferTargetProvider(nextTarget);
    setTransferTargetSkillName(skillDirName(selectedSkill));
  }, [selectedSkill?.id]);

  const resetZipDropState = () => {
    setZipDropState('idle');
    setZipDropMessage(zipDropIdleMessage);
  };

  const applyImportedPackage = (
    summary: ImportedSkillPackageSummary,
    noticeMessage: string,
  ) => {
    resetZipDropState();
    setImportedPackages((current) => [
      summary,
      ...current.filter((item) => item.packageId !== summary.packageId),
    ]);
    setSelectedPackageId(summary.packageId);
    setPackageTargetSkillName(summary.skillDirName);
    setZipPathDraft('');
    onNotice(noticeMessage);
    onError(null);
  };

  const importZipByPath = (
    rawZipPath: string,
    noticeMessage = '技能压缩包已导入到 PromptHarbor 资产库',
  ) => {
    const zipPath = rawZipPath.trim();
    if (!zipPath) {
      onError('请输入 zip 文件路径');
      return;
    }

    setZipPathDraft(zipPath);
    setImportingZip(true);
    api.importSkillZip<ImportedSkillPackageSummary>({ zipPath })
      .then((summary) => applyImportedPackage(summary, noticeMessage))
      .catch((reason) => onError(String(reason)))
      .finally(() => setImportingZip(false));
  };

  const importZipPackage = () => {
    importZipByPath(zipPathDraft);
  };

  const openZipFilePicker = () => {
    if (importingZip) {
      return;
    }

    resetZipDropState();
    zipFileInputRef.current?.click();
  };

  const importZipFromPicker = async (event: ChangeEvent<HTMLInputElement>) => {
    const input = event.currentTarget;
    const file = input.files?.[0];
    input.value = '';

    if (!file) {
      return;
    }
    if (!file.name.toLowerCase().endsWith('.zip')) {
      onError('只支持导入 .zip 压缩包');
      return;
    }

    setImportingZip(true);
    try {
      const zipBytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const summary = await api.importSkillZipBytes<ImportedSkillPackageSummary>({
        originalFileName: file.name,
        zipBytes,
      });
      applyImportedPackage(summary, '技能压缩包已从文件资源管理器导入');
    } catch (reason) {
      onError(String(reason));
    } finally {
      setImportingZip(false);
    }
  };

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    getCurrentWindow()
      .onDragDropEvent((event) => {
        if (disposed) {
          return;
        }

        if (event.payload.type === 'leave') {
          resetZipDropState();
          return;
        }
        if (event.payload.type === 'over') {
          return;
        }

        const candidate = parseZipDropPayload(event.payload.paths);
        if (!candidate.ok) {
          setZipDropState('reject');
          setZipDropMessage(candidate.message);
          if (event.payload.type === 'drop') {
            onErrorRef.current(candidate.message);
            resetZipDropState();
          }
          return;
        }

        if (event.payload.type === 'enter') {
          setZipDropState('accept');
          setZipDropMessage(`松手后导入：${candidate.fileName}`);
          return;
        }

        if (importingZipRef.current) {
          onErrorRef.current('当前已有导入任务进行中，请稍候');
          resetZipDropState();
          return;
        }

        resetZipDropState();
        importZipByPath(candidate.zipPath, '技能压缩包已通过拖拽导入');
      })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
      })
      .catch((reason) => onErrorRef.current(String(reason)));

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const openSelectedSkillDirectory = () => {
    if (!selectedSkill) {
      return;
    }

    api.openProjectPath({ path: selectedSkill.skillDir })
      .then(() => {
        onError(null);
        onNotice('技能目录已打开');
      })
      .catch((reason) => onError(String(reason)));
  };

  const openSelectedPackageDirectory = () => {
    if (!selectedPackage) {
      return;
    }

    api.openProjectPath({ path: selectedPackage.stagedSkillDir })
      .then(() => {
        onError(null);
        onNotice('资产库包目录已打开');
      })
      .catch((reason) => onError(String(reason)));
  };

  const translateSelectedSkill = () => {
    if (!selectedSkill || !selectedTranslationProviderId) {
      return;
    }

    const force = Boolean(selectedSkill.translatedAt);
    setTranslatingSkillId(selectedSkill.id);
    api.translateSkill<SkillTranslationResult>({
      providerId: selectedTranslationProviderId,
      skillId: selectedSkill.id,
      skillFile: selectedSkill.skillFile,
      contentHash: selectedSkill.contentHash,
      force,
    })
      .then((result) => {
        setSkills((current) =>
          current.map((item) =>
            item.id === selectedSkill.id
              ? {
                  ...item,
                  translatedName: result.translatedName,
                  translatedDescription: result.translatedDescription,
                  translatedAt: result.updatedAt,
                  translatedProviderName: result.providerName,
                }
              : item,
          ),
        );
        onNotice(result.cached ? '已使用缓存翻译' : `已更新翻译缓存：${result.providerName}`);
        onError(null);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setTranslatingSkillId(null));
  };

  const exportSelectedSkill = () => {
    if (!selectedSkill) {
      return;
    }

    setExportingSkillId(selectedSkill.id);
    api.exportSkillToLibrary<ImportedSkillPackageSummary>({
      skillId: selectedSkill.id,
      skillFile: selectedSkill.skillFile,
      contentHash: selectedSkill.contentHash,
    })
      .then((summary) => applyImportedPackage(summary, '技能已导出到 PromptHarbor 资产库'))
      .catch((reason) => onError(String(reason)))
      .finally(() => setExportingSkillId(null));
  };

  const installSelectedPackage = () => {
    if (!selectedPackage) {
      return;
    }

    const targets = [
      packageInstallTargets.claude ? 'claude' : null,
      packageInstallTargets.codex ? 'codex' : null,
    ].filter(Boolean) as SkillProvider[];
    if (!targets.length) {
      onError('请至少选择一个安装目标');
      return;
    }

    setInstallingPackageId(selectedPackage.packageId);
    api.installImportedSkill<SkillInstallResult>({
      packageId: selectedPackage.packageId,
      targets,
      targetSkillName: packageTargetSkillName.trim() || null,
      overwrite: false,
    })
      .then((result) => {
        if (result.requiresConfirmation && result.conflicts.length) {
          const conflictList = result.conflicts.map((item) => item.targetDir).join('\n');
          const confirmed = window.confirm(
            `以下目标已存在同名技能目录：\n\n${conflictList}\n\n是否覆盖这些目录？`,
          );
          if (!confirmed) {
            onError(null);
            return result;
          }

          return api.installImportedSkill<SkillInstallResult>({
            packageId: selectedPackage.packageId,
            targets,
            targetSkillName: packageTargetSkillName.trim() || null,
            overwrite: true,
          });
        }

        return result;
      })
      .then((result) => {
        if (!result || !result.installed) {
          return;
        }

        onNotice(result.message);
        onError(null);
        loadSkills('refresh', selectedProvider);
        loadImportedPackages();
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setInstallingPackageId(null));
  };

  const transferSelectedSkill = () => {
    if (!selectedSkill || selectedSkill.sourceKind !== 'user') {
      return;
    }

    const targetProvider = transferTargetProvider;
    const targetLabel = providerLabel(targetProvider);
    const confirmed = window.confirm(
      `把技能「${selectedSkill.name}」从 ${selectedSkill.providerLabel} 转移到 ${targetLabel}？\n\n转移完成后，源目录会被删除。`,
    );
    if (!confirmed) {
      return;
    }

    setTransferringSkillId(selectedSkill.id);
    api.transferSkill<SkillTransferResult>({
      skillId: selectedSkill.id,
      skillFile: selectedSkill.skillFile,
      contentHash: selectedSkill.contentHash,
      targetProvider,
      targetSkillName: transferTargetSkillName.trim() || null,
      overwrite: false,
    })
      .then((result) => {
        if (result.requiresConfirmation && result.conflicts.length) {
          const conflictList = result.conflicts.map((item) => item.targetDir).join('\n');
          const overwriteConfirmed = window.confirm(
            `以下目标已存在同名技能目录：\n\n${conflictList}\n\n是否覆盖这些目录并继续转移？`,
          );
          if (!overwriteConfirmed) {
            onError(null);
            return result;
          }

          return api.transferSkill<SkillTransferResult>({
            skillId: selectedSkill.id,
            skillFile: selectedSkill.skillFile,
            contentHash: selectedSkill.contentHash,
            targetProvider,
            targetSkillName: transferTargetSkillName.trim() || null,
            overwrite: true,
          });
        }

        return result;
      })
      .then((result) => {
        if (!result || !result.transferred) {
          return;
        }

        onNotice(result.message);
        onError(null);
        loadSkills('refresh', targetProvider);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setTransferringSkillId(null));
  };

  const deleteSelectedSkill = () => {
    if (!selectedSkill || selectedSkill.sourceKind !== 'user') {
      return;
    }

    const confirmed = window.confirm(
      `删除技能「${selectedSkill.name}」？\n\n这会直接删除 ${selectedSkill.providerLabel} 下的技能目录，无法恢复。`,
    );
    if (!confirmed) {
      return;
    }

    setDeletingSkillId(selectedSkill.id);
    api.deleteSkill<SkillDeleteResult>({
      skillId: selectedSkill.id,
      skillFile: selectedSkill.skillFile,
      contentHash: selectedSkill.contentHash,
    })
      .then((result) => {
        if (!result.deleted) {
          return;
        }
        onNotice(result.message);
        onError(null);
        loadSkills('refresh', selectedProvider);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setDeletingSkillId(null));
  };

  const deleteSelectedPackage = () => {
    if (!selectedPackage) {
      return;
    }

    const confirmed = window.confirm(
      `删除资产库包「${selectedPackage.name}」？\n\n这不会删除已经安装到 Claude Code 或 Codex CLI 的副本。`,
    );
    if (!confirmed) {
      return;
    }

    setDeletingPackageId(selectedPackage.packageId);
    api.deleteImportedSkillPackage<ImportedSkillPackageDeleteResult>({
      packageId: selectedPackage.packageId,
    })
      .then((result) => {
        if (!result.deleted) {
          return;
        }
        onNotice(result.message);
        onError(null);
        loadImportedPackages();
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => setDeletingPackageId(null));
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto pr-1 xl:overflow-hidden xl:pr-0">
      <section className="shrink-0 rounded-[22px] border border-border/50 bg-card/95 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-4 px-5 py-5">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Sparkles className="text-primary" size={18} />
              <h2 className="text-xl font-bold">技能管理</h2>
            </div>
            <p className="max-w-3xl text-sm leading-6 text-muted-foreground">
              把 Claude Code、Codex CLI 和 PromptHarbor 资产库里的技能放到一张工作台上统一查看、翻译、备份、安装、转移和删除。
            </p>
          </div>
          <button
            className="inline-flex items-center gap-2 rounded-xl border border-border bg-background px-3 py-2 text-sm font-semibold text-foreground transition-colors hover:bg-secondary disabled:cursor-default disabled:opacity-60"
            disabled={loading || refreshing}
            onClick={() => {
              loadSkills('refresh', selectedProvider);
              loadImportedPackages();
              loadProviders();
            }}
            type="button"
          >
            <RefreshCw className={cn(refreshing ? 'animate-spin' : '')} size={15} />
            刷新
          </button>
        </div>

        <div className="grid gap-3 border-t border-border/40 px-5 py-4 sm:grid-cols-2 xl:grid-cols-4">
          <MetricTile
            label="当前技能数"
            value={`${providerSkills.length}`}
            hint={`${providerLabel(selectedProvider)} 技能总数`}
          />
          <MetricTile
            label="用户技能"
            value={`${providerUserSkillCount}`}
            hint="支持转移与删除"
          />
          <MetricTile
            label="缓存翻译"
            value={`${providerTranslatedCount}`}
            hint="按技能内容哈希缓存"
          />
          <MetricTile
            label="资产库"
            value={`${importedPackages.length}`}
            hint="PromptHarbor 备份包"
          />
        </div>

        <div className="flex flex-wrap gap-2 border-t border-border/40 px-5 py-4">
          {providerOrder.map((provider) => {
            const active = provider === selectedProvider;
            return (
              <button
                className={cn(
                  'inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-sm font-semibold transition-all',
                  active
                    ? 'border-primary bg-primary text-white shadow-sm'
                    : 'border-border bg-background text-foreground hover:bg-secondary',
                )}
                key={provider}
                onClick={() => setSelectedProvider(provider)}
                type="button"
              >
                <Bot size={15} />
                <span>{providerLabel(provider)}</span>
                <span
                  className={cn(
                    'rounded-md px-1.5 py-0.5 text-[10px] font-bold',
                    active ? 'bg-white/20 text-white' : 'bg-secondary text-muted-foreground',
                  )}
                >
                  {providerCounts[provider]}
                </span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="grid grid-cols-1 gap-4 xl:min-h-0 xl:flex-1 xl:grid-cols-[320px_minmax(0,1fr)_360px]">
        <aside className="flex flex-col rounded-[22px] border border-border/50 bg-card shadow-sm xl:min-h-0 xl:overflow-hidden">
          <div className="border-b border-border/40 px-4 py-4">
            <div className="text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
              已安装技能
            </div>
            <p className="mt-1 mb-0 text-sm text-muted-foreground">
              {loading ? '读取中…' : `${providerSkills.length} 个技能 · ${providerReadonlySkillCount} 个只读`}
            </p>
          </div>

          <div className="px-3 py-3 xl:min-h-0 xl:flex-1 xl:overflow-y-auto">
            {loading ? (
              <EmptyPanel
                icon={<RefreshCw className="animate-spin text-muted-foreground" size={18} />}
                title="正在读取技能目录"
                body="读取 Claude Code 和 Codex CLI 的本地技能。"
              />
            ) : groupedSkills.length ? (
              <div className="space-y-4">
                {groupedSkills.map((group) => (
                  <div className="space-y-2" key={group.sourceKind}>
                    <div className="px-1 text-[11px] font-bold uppercase tracking-[0.22em] text-muted-foreground">
                      {sourceSectionLabel(group.sourceKind)}
                    </div>
                    <div className="space-y-2">
                      {group.items.map((skill) => {
                        const active = selectedSkill?.id === skill.id;
                        return (
                          <button
                            className={cn(
                              'w-full rounded-2xl border p-3 text-left transition-all',
                              active
                                ? 'border-primary/35 bg-primary/[0.06] shadow-sm'
                                : 'border-border/60 bg-background hover:border-primary/25 hover:bg-secondary/40',
                            )}
                            key={skill.id}
                            onClick={() => setSelectedSkillId(skill.id)}
                            type="button"
                          >
                            <div className="flex items-start justify-between gap-3">
                              <div className="min-w-0 flex-1">
                                <div className="truncate text-sm font-bold text-foreground">
                                  {skill.name}
                                </div>
                                <div className="mt-1 text-[11px] text-muted-foreground">
                                  {skill.sourceLabel}
                                </div>
                              </div>
                              <span
                                className={cn(
                                  'shrink-0 rounded-full px-2 py-1 text-[10px] font-bold',
                                  skill.sourceKind === 'user'
                                    ? 'bg-emerald-50 text-emerald-700'
                                    : 'bg-slate-100 text-slate-600',
                                )}
                              >
                                {skill.sourceKind === 'user' ? '可管理' : '只读'}
                              </span>
                            </div>
                            <p className="mt-2 mb-0 text-xs leading-5 text-muted-foreground">
                              {compactText(skill.translatedDescription || skill.description || '未提供描述', 120)}
                            </p>
                            <div className="mt-3 flex items-center justify-between gap-3">
                              <div className="truncate text-[11px] text-primary/85">
                                {skill.translatedName || '未缓存中文标题'}
                              </div>
                              {skill.translatedAt ? (
                                <span className="inline-flex items-center gap-1 text-[10px] font-semibold text-emerald-700">
                                  <CheckCircle2 size={11} />
                                  已译
                                </span>
                              ) : null}
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <EmptyPanel
                icon={<Sparkles className="text-muted-foreground" size={18} />}
                title="当前提供者下没有技能"
                body="切换到另一个提供者，或先导入一份技能包。"
              />
            )}
          </div>
        </aside>

        <div className="xl:min-h-0 xl:overflow-y-auto">
          {!selectedSkill ? (
            <div className="flex h-full min-h-[420px] items-center justify-center rounded-[22px] border border-border/50 bg-card px-8 text-center shadow-sm">
              <div>
                <div className="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-2xl bg-secondary text-muted-foreground">
                  <FileText size={22} />
                </div>
                <p className="mb-1 text-base font-bold text-foreground">选择一个技能查看详情</p>
                <p className="mb-0 text-sm text-muted-foreground">
                  中间区域会显示说明、翻译状态和技能管理动作。
                </p>
              </div>
            </div>
          ) : (
            <div className="space-y-4">
              <section className="rounded-[22px] border border-border/50 bg-card shadow-sm">
                <div className="flex flex-wrap items-start justify-between gap-4 border-b border-border/40 px-5 py-5">
                  <div className="min-w-0 space-y-2">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="truncate text-[28px] font-bold leading-none text-foreground">
                        {selectedSkill.translatedName || selectedSkill.name}
                      </h3>
                      <span className="rounded-full border border-border bg-background px-2.5 py-1 text-[11px] font-bold text-muted-foreground">
                        {selectedSkill.providerLabel}
                      </span>
                      <span className="rounded-full border border-border bg-background px-2.5 py-1 text-[11px] font-bold text-muted-foreground">
                        {selectedSkill.sourceLabel}
                      </span>
                      {selectedSkill.sourceKind !== 'user' ? (
                        <span className="inline-flex items-center gap-1 rounded-full bg-slate-100 px-2.5 py-1 text-[11px] font-bold text-slate-700">
                          <Shield size={12} />
                          只读
                        </span>
                      ) : null}
                    </div>
                    <p className="mb-0 text-sm leading-6 text-muted-foreground">
                      {selectedSkill.translatedDescription || selectedSkill.description || '未提供描述'}
                    </p>
                    <p className="mb-0 truncate text-xs text-muted-foreground" title={selectedSkill.skillDir}>
                      {selectedSkill.skillDir}
                    </p>
                  </div>

                  <button
                    className="secondary-action inline-flex items-center gap-2"
                    onClick={openSelectedSkillDirectory}
                    type="button"
                  >
                    <FolderOpen size={15} />
                    打开目录
                  </button>
                </div>

                <div className="grid gap-4 px-5 py-5 lg:grid-cols-2">
                  <div className="rounded-2xl border border-border/50 bg-background p-4">
                    <div className="mb-3 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                      <Languages size={14} />
                      翻译缓存
                    </div>
                    <p className="mb-1 text-sm font-semibold text-foreground">
                      {selectedSkill.translatedName || '未缓存中文标题'}
                    </p>
                    <p className="mb-3 text-sm leading-6 text-muted-foreground">
                      {selectedSkill.translatedDescription || '还没有中文摘要，可手动生成并缓存。'}
                    </p>
                    {selectedSkill.translatedAt ? (
                      <p className="mb-3 text-xs text-muted-foreground">
                        最近缓存：{selectedSkill.translatedProviderName || '未知供应商'} ·{' '}
                        {formatTimestamp(selectedSkill.translatedAt)}
                      </p>
                    ) : null}
                    <div className="space-y-3">
                      <label className="block">
                        <span className="mb-1 block text-xs font-semibold text-muted-foreground">
                          翻译供应商
                        </span>
                        <select
                          className="h-10 w-full rounded-xl border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
                          disabled={providersLoading || !translationProviders.length}
                          onChange={(event) => setSelectedTranslationProviderId(event.currentTarget.value)}
                          value={selectedTranslationProviderId}
                        >
                          {translationProviders.length ? (
                            translationProviders.map((provider) => (
                              <option key={provider.id} value={provider.id}>
                                {provider.name} · {provider.defaultModel}
                              </option>
                            ))
                          ) : (
                            <option value="">
                              {providersLoading ? '读取供应商中…' : '没有可用的已启用供应商'}
                            </option>
                          )}
                        </select>
                      </label>
                      <button
                        className="primary-action w-full"
                        disabled={!canTranslate}
                        onClick={translateSelectedSkill}
                        type="button"
                      >
                        {translatingSkillId === selectedSkill.id
                          ? '翻译中'
                          : selectedSkill.translatedAt
                            ? '更新翻译'
                            : '翻译并缓存'}
                      </button>
                      <p className="mb-0 text-xs leading-5 text-muted-foreground">
                        同一份技能内容会优先命中缓存；已有缓存时点击按钮会重新翻译并覆盖。
                      </p>
                    </div>
                  </div>

                  <div className="rounded-2xl border border-border/50 bg-background p-4">
                    <div className="mb-3 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                      <Archive size={14} />
                      管理动作
                    </div>

                    <div className="space-y-4">
                      <div className="rounded-xl border border-border/50 bg-muted/20 p-3">
                        <div className="mb-2 text-sm font-semibold text-foreground">导出到资产库</div>
                        <p className="mb-3 text-xs leading-5 text-muted-foreground">
                          给当前技能再存一份备份包，后续可以重复安装到 Claude Code 或 Codex CLI。
                        </p>
                        <button
                          className="secondary-action w-full"
                          disabled={exportingSkillId === selectedSkill.id}
                          onClick={exportSelectedSkill}
                          type="button"
                        >
                          {exportingSkillId === selectedSkill.id ? '导出中' : '备份到 PromptHarbor 资产库'}
                        </button>
                      </div>

                      {selectedSkill.sourceKind === 'user' ? (
                        <>
                          <div className="rounded-xl border border-border/50 bg-muted/20 p-3">
                            <div className="mb-2 flex items-center gap-2 text-sm font-semibold text-foreground">
                              <ArrowRightLeft size={14} />
                              转移到另一个提供者
                            </div>
                            <div className="grid gap-3">
                              <label className="block">
                                <span className="mb-1 block text-xs font-semibold text-muted-foreground">
                                  目标提供者
                                </span>
                                <select
                                  className="h-10 w-full rounded-xl border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
                                  onChange={(event) =>
                                    setTransferTargetProvider(event.currentTarget.value as SkillProvider)
                                  }
                                  value={transferTargetProvider}
                                >
                                  {providerOrder
                                    .filter((provider) => provider !== selectedSkill.provider)
                                    .map((provider) => (
                                      <option key={provider} value={provider}>
                                        {providerLabel(provider)}
                                      </option>
                                    ))}
                                </select>
                              </label>
                              <label className="block">
                                <span className="mb-1 block text-xs font-semibold text-muted-foreground">
                                  转移后的目录名
                                </span>
                                <input
                                  className="h-10 w-full rounded-xl border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
                                  onChange={(event) => setTransferTargetSkillName(event.currentTarget.value)}
                                  value={transferTargetSkillName}
                                />
                              </label>
                              <button
                                className="primary-action w-full"
                                disabled={!canTransferSelectedSkill}
                                onClick={transferSelectedSkill}
                                type="button"
                              >
                                {transferringSkillId === selectedSkill.id ? '转移中' : '开始转移'}
                              </button>
                            </div>
                          </div>

                          <div className="rounded-xl border border-rose-200 bg-rose-50 p-3">
                            <div className="mb-2 flex items-center gap-2 text-sm font-semibold text-rose-700">
                              <Trash2 size={14} />
                              删除当前技能
                            </div>
                            <p className="mb-3 text-xs leading-5 text-rose-700/90">
                              只删除当前提供者下的用户技能目录，不会动到资产库里的备份包。
                            </p>
                            <button
                              className="inline-flex min-h-[34px] w-full items-center justify-center gap-2 rounded-xl border border-rose-200 bg-white px-3 py-2 text-sm font-semibold text-rose-700 transition-colors hover:bg-rose-100 disabled:cursor-default disabled:opacity-60"
                              disabled={deletingSkillId === selectedSkill.id}
                              onClick={deleteSelectedSkill}
                              type="button"
                            >
                              {deletingSkillId === selectedSkill.id ? '删除中' : '删除技能'}
                            </button>
                          </div>
                        </>
                      ) : (
                        <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700">
                          <div className="mb-2 flex items-center gap-2 font-semibold">
                            <Shield size={14} />
                            当前技能为只读来源
                          </div>
                          <p className="mb-0 text-xs leading-5 text-slate-600">
                            系统内置或其他非用户来源技能不支持直接删除和转移，但可以先备份到资产库，再安装成一份可管理副本。
                          </p>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              </section>

              <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px]">
                <div className="overflow-hidden rounded-[22px] border border-border/50 bg-card shadow-sm xl:min-h-0">
                  <div className="flex items-center justify-between gap-3 border-b border-border/40 px-4 py-4">
                    <div>
                      <div className="text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                        SKILL.md
                      </div>
                      <p className="mt-1 mb-0 truncate text-xs text-muted-foreground" title={selectedSkill.skillFile}>
                        {selectedSkill.skillFile}
                      </p>
                    </div>
                    {detailLoading ? (
                      <span className="text-xs font-semibold text-muted-foreground">读取中…</span>
                    ) : null}
                  </div>
                  <div className="max-h-[calc(100vh-360px)] overflow-auto px-4 py-4">
                    <pre className="m-0 whitespace-pre-wrap break-words font-mono text-xs leading-6 text-foreground">
                      {selectedSkillDetail?.contentMd || '正在读取技能说明内容…'}
                    </pre>
                  </div>
                </div>

                <div className="space-y-4">
                  <div className="rounded-[22px] border border-border/50 bg-card p-4 shadow-sm">
                    <div className="mb-3 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                      技能信息
                    </div>
                    <div className="space-y-3 text-sm text-foreground">
                      <InfoRow label="原始名称" value={selectedSkill.name} />
                      <InfoRow label="本地状态" value={selectedSkill.localStatus} />
                      <InfoRow label="相对路径" value={selectedSkill.relativePath} mono />
                      <InfoRow label="内容哈希" value={selectedSkill.contentHash} mono />
                    </div>
                  </div>

                  <div className="rounded-[22px] border border-border/50 bg-card p-4 shadow-sm">
                    <div className="mb-3 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                      原始描述
                    </div>
                    <p className="mb-0 text-sm leading-6 text-muted-foreground">
                      {selectedSkill.description || '未提供原始描述'}
                    </p>
                  </div>
                </div>
              </section>
            </div>
          )}
        </div>

        <aside className="flex flex-col rounded-[22px] border border-border/50 bg-card shadow-sm xl:min-h-0 xl:overflow-hidden">
          <div className="border-b border-border/40 px-4 py-4">
            <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
              <Boxes size={14} />
              PromptHarbor 资产库
            </div>
            <p className="mt-1 mb-0 text-sm text-muted-foreground">
              导入 zip、保存备份包，并把它们安装到 Claude Code 或 Codex CLI。
            </p>
          </div>

          <div className="px-4 py-4 xl:min-h-0 xl:flex-1 xl:overflow-y-auto">
            <div className="space-y-4">
              <div className="rounded-2xl border border-border/50 bg-background p-4">
                <div className="mb-3 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                  <Upload size={14} />
                  导入 Zip
                </div>

                <input
                  accept=".zip,application/zip"
                  className="hidden"
                  onChange={importZipFromPicker}
                  ref={zipFileInputRef}
                  type="file"
                />

                <div
                  className={cn(
                    'rounded-2xl border border-dashed p-3 transition-colors',
                    zipDropState === 'accept'
                      ? 'border-emerald-300 bg-emerald-50'
                      : zipDropState === 'reject'
                        ? 'border-rose-300 bg-rose-50'
                        : 'border-border/60 bg-muted/20',
                  )}
                >
                  <div className="flex items-start gap-3">
                    <div
                      className={cn(
                        'rounded-xl p-2',
                        zipDropState === 'accept'
                          ? 'bg-emerald-100 text-emerald-700'
                          : zipDropState === 'reject'
                            ? 'bg-rose-100 text-rose-700'
                            : 'bg-background text-primary',
                      )}
                    >
                      <FolderOpen size={16} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-semibold text-foreground">
                        文件资源管理器 / 直接拖拽
                      </div>
                      <p
                        className={cn(
                          'mt-1 mb-0 text-xs leading-5',
                          zipDropState === 'reject' ? 'text-rose-700' : 'text-muted-foreground',
                        )}
                      >
                        {zipDropMessage}
                      </p>
                    </div>
                  </div>
                  <button
                    className="secondary-action mt-3 w-full"
                    disabled={importingZip}
                    onClick={openZipFilePicker}
                    type="button"
                  >
                    {importingZip ? '导入中' : '选择 zip 文件'}
                  </button>
                </div>

                <div className="mt-3 space-y-3">
                  <label className="block">
                    <span className="mb-1 block text-xs font-semibold text-muted-foreground">
                      手动路径
                    </span>
                    <input
                      className="h-10 w-full rounded-xl border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
                      onChange={(event) => setZipPathDraft(event.currentTarget.value)}
                      placeholder="例如：D:\\skills\\my-skill.zip"
                      value={zipPathDraft}
                    />
                  </label>
                  <button
                    className="secondary-action w-full"
                    disabled={importingZip}
                    onClick={importZipPackage}
                    type="button"
                  >
                    {importingZip ? '导入中' : '按路径导入'}
                  </button>
                  <p className="mb-0 text-xs leading-5 text-muted-foreground">
                    导入后会同时保留原始 zip 和解压后的资产库副本。
                  </p>
                </div>
              </div>

              <div className="rounded-2xl border border-border/50 bg-background p-4">
                <div className="mb-3 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                  <FileArchive size={14} />
                  资产库包列表
                </div>

                {packagesLoading ? (
                  <EmptyPanel
                    icon={<RefreshCw className="animate-spin text-muted-foreground" size={16} />}
                    title="正在读取资产库"
                    body="读取 PromptHarbor 本地保存的技能包。"
                    compact
                  />
                ) : importedPackages.length ? (
                  <div className="space-y-2">
                    {importedPackages.map((item) => {
                      const active = selectedPackage?.packageId === item.packageId;
                      return (
                        <button
                          className={cn(
                            'w-full rounded-2xl border p-3 text-left transition-all',
                            active
                              ? 'border-primary/35 bg-primary/[0.06]'
                              : 'border-border/60 bg-white hover:bg-secondary/40',
                          )}
                          key={item.packageId}
                          onClick={() => setSelectedPackageId(item.packageId)}
                          type="button"
                        >
                          <div className="flex items-start justify-between gap-3">
                            <div className="min-w-0">
                              <div className="truncate text-sm font-semibold text-foreground">
                                {item.name}
                              </div>
                              <div className="mt-1 truncate text-[11px] text-muted-foreground">
                                {item.originalFileName}
                              </div>
                            </div>
                            <span
                              className={cn(
                                'shrink-0 rounded-full px-2 py-1 text-[10px] font-bold',
                                item.installedTargets.length
                                  ? 'bg-emerald-50 text-emerald-700'
                                  : 'bg-secondary text-muted-foreground',
                              )}
                            >
                              {item.installedTargets.length ? '已安装' : '未安装'}
                            </span>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                ) : (
                  <EmptyPanel
                    icon={<PackagePlus className="text-muted-foreground" size={16} />}
                    title="资产库还是空的"
                    body="导入一个 zip，或把左侧已安装技能备份到资产库。"
                    compact
                  />
                )}
              </div>

              {selectedPackage ? (
                <div className="rounded-2xl border border-border/50 bg-background p-4">
                  <div className="mb-3 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-muted-foreground">
                    <PackagePlus size={14} />
                    资产库包操作
                  </div>

                  <div className="space-y-4">
                    <div>
                      <div className="text-sm font-semibold text-foreground">{selectedPackage.name}</div>
                      <p className="mt-1 mb-0 text-xs leading-5 text-muted-foreground">
                        {selectedPackage.description || '未提供描述'}
                      </p>
                      <p className="mt-2 mb-0 text-xs text-muted-foreground">
                        导入时间：{formatTimestamp(selectedPackage.importedAt)}
                      </p>
                    </div>

                    <label className="block">
                      <span className="mb-1 block text-xs font-semibold text-muted-foreground">
                        安装后的目录名
                      </span>
                      <input
                        className="h-10 w-full rounded-xl border border-border bg-background px-3 text-sm text-foreground outline-none focus:border-primary focus:ring-2 focus:ring-primary/15"
                        onChange={(event) => setPackageTargetSkillName(event.currentTarget.value)}
                        value={packageTargetSkillName}
                      />
                    </label>

                    <div className="grid gap-2 rounded-xl border border-border/50 bg-muted/20 p-3">
                      <label className="flex items-center gap-2 text-sm text-foreground">
                        <input
                          checked={packageInstallTargets.claude}
                          onChange={(event) =>
                            setPackageInstallTargets((current) => ({
                              ...current,
                              claude: event.currentTarget.checked,
                            }))
                          }
                          type="checkbox"
                        />
                        安装到 Claude Code
                      </label>
                      <label className="flex items-center gap-2 text-sm text-foreground">
                        <input
                          checked={packageInstallTargets.codex}
                          onChange={(event) =>
                            setPackageInstallTargets((current) => ({
                              ...current,
                              codex: event.currentTarget.checked,
                            }))
                          }
                          type="checkbox"
                        />
                        安装到 Codex CLI
                      </label>
                    </div>

                    <button
                      className="primary-action w-full"
                      disabled={!canInstallSelectedPackage}
                      onClick={installSelectedPackage}
                      type="button"
                    >
                      {installingPackageId === selectedPackage.packageId ? '安装中' : '安装到所选目标'}
                    </button>

                    <div className="grid gap-2 sm:grid-cols-2">
                      <button
                        className="secondary-action"
                        onClick={openSelectedPackageDirectory}
                        type="button"
                      >
                        打开资产目录
                      </button>
                      <button
                        className="inline-flex min-h-[34px] items-center justify-center gap-2 rounded-xl border border-rose-200 bg-white px-3 py-2 text-sm font-semibold text-rose-700 transition-colors hover:bg-rose-100 disabled:cursor-default disabled:opacity-60"
                        disabled={deletingPackageId === selectedPackage.packageId}
                        onClick={deleteSelectedPackage}
                        type="button"
                      >
                        {deletingPackageId === selectedPackage.packageId ? '删除中' : '删除资产包'}
                      </button>
                    </div>

                    {selectedPackage.installedTargets.length ? (
                      <div className="rounded-xl border border-emerald-200 bg-emerald-50 p-3">
                        <div className="mb-2 flex items-center gap-2 text-xs font-bold uppercase tracking-[0.22em] text-emerald-700">
                          <CheckCircle2 size={13} />
                          已安装目标
                        </div>
                        <div className="space-y-2">
                          {selectedPackage.installedTargets.map((target) => (
                            <div className="text-xs leading-5 text-emerald-800" key={target.provider}>
                              {target.providerLabel} · {target.targetDir}
                            </div>
                          ))}
                        </div>
                      </div>
                    ) : null}
                  </div>
                </div>
              ) : null}
            </div>
          </div>
        </aside>
      </section>
    </div>
  );
}

function MetricTile({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint: string;
}) {
  return (
    <div className="rounded-2xl border border-border/50 bg-background/80 px-4 py-3">
      <div className="text-[11px] font-bold uppercase tracking-[0.22em] text-muted-foreground">
        {label}
      </div>
      <div className="mt-2 text-2xl font-bold text-foreground">{value}</div>
      <div className="mt-1 text-xs text-muted-foreground">{hint}</div>
    </div>
  );
}

function EmptyPanel({
  icon,
  title,
  body,
  compact = false,
}: {
  icon: ReactNode;
  title: string;
  body: string;
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        'flex flex-col items-center justify-center text-center',
        compact ? 'min-h-[120px] px-3 py-5' : 'min-h-[220px] px-5',
      )}
    >
      <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-2xl bg-secondary">
        {icon}
      </div>
      <p className="mb-1 text-sm font-bold text-foreground">{title}</p>
      <p className="mb-0 text-xs leading-5 text-muted-foreground">{body}</p>
    </div>
  );
}

function InfoRow({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <div className="text-xs font-semibold text-muted-foreground">{label}</div>
      <div className={cn('mt-1 break-all', mono ? 'font-mono text-xs' : '')}>{value}</div>
    </div>
  );
}

function sourceSectionLabel(sourceKind: SkillSourceKind) {
  if (sourceKind === 'system') {
    return '系统内置';
  }
  if (sourceKind === 'project') {
    return '项目技能';
  }
  return '用户技能';
}

function providerLabel(provider: SkillProvider) {
  return provider === 'claude' ? 'Claude Code' : 'Codex CLI';
}

function otherProvider(provider: SkillProvider): SkillProvider {
  return provider === 'claude' ? 'codex' : 'claude';
}

function skillDirName(skill: SkillListItem) {
  const normalized = skill.relativePath.replace(/\\/g, '/');
  return normalized.split('/').filter(Boolean).pop() || skill.name;
}

function compactText(value: string, maxLength: number) {
  if (value.length <= maxLength) {
    return value;
  }
  return `${value.slice(0, maxLength).trimEnd()}...`;
}

function parseZipDropPayload(
  paths: string[],
):
  | { ok: true; zipPath: string; fileName: string }
  | { ok: false; message: string } {
  if (!paths.length) {
    return { ok: false, message: '没有检测到可导入的文件。' };
  }
  if (paths.length > 1) {
    return { ok: false, message: '一次只能拖入一个 .zip 压缩包。' };
  }

  const zipPath = paths[0]?.trim() ?? '';
  if (!zipPath) {
    return { ok: false, message: '拖入的文件路径无效。' };
  }
  if (!zipPath.toLowerCase().endsWith('.zip')) {
    return { ok: false, message: '只支持拖入 .zip 压缩包。' };
  }

  return {
    ok: true,
    zipPath,
    fileName: zipPath.split(/[/\\]/).pop() || zipPath,
  };
}

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}
