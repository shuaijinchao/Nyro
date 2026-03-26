import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState, useRef } from "react";
import { backend, IS_TAURI } from "@/lib/backend";
import { localizeBackendErrorMessage } from "@/lib/backend-error";
import type { GatewayStatus, ExportData, ImportResult } from "@/lib/types";
import { useLocale } from "@/lib/i18n";
import {
  Download,
  Upload,
  Save,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";

export default function SettingsPage() {
  const { locale } = useLocale();
  const isZh = locale === "zh-CN";
  const appVersion = import.meta.env.VITE_APP_VERSION;

  const qc = useQueryClient();
  const fileRef = useRef<HTMLInputElement>(null);
  const [errorDialog, setErrorDialog] = useState<{ title: string; description?: string } | null>(null);

  const { data: status } = useQuery<GatewayStatus>({
    queryKey: ["gateway-status"],
    queryFn: () => backend("get_gateway_status"),
  });

  const { data: retentionDays } = useQuery<string | null>({
    queryKey: ["setting", "log_retention_days"],
    queryFn: () => backend("get_setting", { key: "log_retention_days" }),
  });
  const { data: proxyEnabledSetting } = useQuery<string | null>({
    queryKey: ["setting", "proxy_enabled"],
    queryFn: () => backend("get_setting", { key: "proxy_enabled" }),
  });
  const { data: proxyUrlSetting } = useQuery<string | null>({
    queryKey: ["setting", "proxy_url"],
    queryFn: () => backend("get_setting", { key: "proxy_url" }),
  });
  const { data: proxyBypassSetting } = useQuery<string | null>({
    queryKey: ["setting", "proxy_bypass"],
    queryFn: () => backend("get_setting", { key: "proxy_bypass" }),
  });

  const [retentionInput, setRetentionInput] = useState<string>("");
  const retentionValue = retentionInput || retentionDays || "30";
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [proxyUrl, setProxyUrl] = useState("");
  const [proxyBypass, setProxyBypass] = useState("");

  useEffect(() => {
    const normalized = (proxyEnabledSetting ?? "").trim().toLowerCase();
    setProxyEnabled(["1", "true", "yes", "on"].includes(normalized));
    setProxyUrl(proxyUrlSetting ?? "");
    setProxyBypass(proxyBypassSetting ?? "");
  }, [proxyEnabledSetting, proxyUrlSetting, proxyBypassSetting]);

  function formatErrorMessage(error: unknown) {
    return localizeBackendErrorMessage(error, isZh);
  }

  function showErrorDialog(titleZh: string, titleEn: string, error: unknown) {
    setErrorDialog({
      title: isZh ? titleZh : titleEn,
      description: formatErrorMessage(error),
    });
  }

  const saveSetting = useMutation({
    mutationFn: (value: string) =>
      backend("set_setting", { key: "log_retention_days", value }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["setting", "log_retention_days"] }),
    onError: (error: unknown) => {
      showErrorDialog("保存设置失败", "Failed to save settings", error);
    },
  });
  const saveProxySettings = useMutation({
    mutationFn: async () => {
      await Promise.all([
        backend("set_setting", { key: "proxy_enabled", value: proxyEnabled ? "true" : "false" }),
        backend("set_setting", { key: "proxy_url", value: proxyUrl.trim() }),
        backend("set_setting", { key: "proxy_bypass", value: proxyBypass.trim() }),
      ]);
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["setting", "proxy_enabled"] });
      qc.invalidateQueries({ queryKey: ["setting", "proxy_url"] });
      qc.invalidateQueries({ queryKey: ["setting", "proxy_bypass"] });
    },
    onError: (error: unknown) => {
      showErrorDialog("保存代理设置失败", "Failed to save proxy settings", error);
    },
  });

  const exportMut = useMutation({
    mutationFn: () => backend<ExportData>("export_config"),
    onSuccess: (data) => {
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `nyro-config-${new Date().toISOString().slice(0, 10)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    },
    onError: (error: unknown) => {
      showErrorDialog("导出配置失败", "Failed to export config", error);
    },
  });

  const importMut = useMutation({
    mutationFn: (data: ExportData) =>
      backend<ImportResult>("import_config", { data }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["providers"] });
      qc.invalidateQueries({ queryKey: ["routes"] });
    },
    onError: (error: unknown) => {
      showErrorDialog("导入配置失败", "Failed to import config", error);
    },
  });

  function handleImportFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const data = JSON.parse(reader.result as string) as ExportData;
        importMut.mutate(data);
      } catch {
        setErrorDialog({
          title: isZh ? "导入配置失败" : "Failed to import config",
          description: isZh ? "无效的 JSON 文件" : "Invalid JSON file",
        });
      }
    };
    reader.readAsText(file);
    e.target.value = "";
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-slate-900">{isZh ? "设置" : "Settings"}</h1>
        <p className="mt-1 text-sm text-slate-500">
          {isZh ? "网关配置" : "Gateway configuration"}
        </p>
      </div>

      {/* Gateway Status */}
      <div className="glass rounded-2xl p-6 space-y-4">
        <h2 className="text-lg font-semibold text-slate-900">{isZh ? "网关状态" : "Gateway Status"}</h2>
        <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
          <div className="rounded-xl bg-slate-50 p-4">
            <p className="text-xs text-slate-500">{isZh ? "状态" : "Status"}</p>
            <p className="mt-1 font-semibold text-green-600">{status?.status ?? "–"}</p>
          </div>
          <div className="rounded-xl bg-slate-50 p-4">
            <p className="text-xs text-slate-500">{isZh ? "代理端口" : "Proxy Port"}</p>
            <p className="mt-1 font-semibold text-slate-900">{status?.proxy_port ?? "–"}</p>
          </div>
          <div className="rounded-xl bg-slate-50 p-4">
            <p className="text-xs text-slate-500">{isZh ? "模式" : "Mode"}</p>
            <p className="mt-1 font-semibold text-slate-900">{IS_TAURI ? (isZh ? "桌面版" : "Desktop") : "Server"}</p>
          </div>
          <div className="rounded-xl bg-slate-50 p-4">
            <p className="text-xs text-slate-500">{isZh ? "版本" : "Version"}</p>
            <p className="mt-1 font-semibold text-slate-900">{appVersion}</p>
          </div>
        </div>
      </div>

      {/* Log Retention & Config */}
      <div className="glass rounded-2xl p-6 space-y-5">
        <h2 className="text-lg font-semibold text-slate-900">{isZh ? "配置" : "Configuration"}</h2>

        <div className="grid grid-cols-1 gap-5 md:grid-cols-2">
          {/* Log Retention */}
          <div className="rounded-xl bg-slate-50 p-4 space-y-3">
            <div>
              <p className="text-sm font-medium text-slate-700">{isZh ? "日志保留" : "Log Retention"}</p>
              <p className="text-xs text-slate-500">
                {isZh ? "自动删除超过 N 天的日志" : "Auto-delete logs older than N days"}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Input
                type="number"
                min={1}
                max={365}
                value={retentionValue}
                onChange={(e) => setRetentionInput(e.target.value)}
                className="w-24"
              />
              <span className="text-sm text-slate-500">{isZh ? "天" : "days"}</span>
              <Button
                onClick={() => saveSetting.mutate(retentionValue)}
                disabled={saveSetting.isPending}
                size="sm"
                className="ml-auto flex items-center gap-1.5"
              >
                {saveSetting.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Save className="h-3.5 w-3.5" />
                )}
                {isZh ? "保存" : "Save"}
              </Button>
            </div>
            {saveSetting.isSuccess && (
              <p className="text-xs text-green-600">{isZh ? "保存成功" : "Saved successfully"}</p>
            )}
          </div>

          {/* Import / Export */}
          <div className="rounded-xl bg-slate-50 p-4 space-y-3">
            <div>
              <p className="text-sm font-medium text-slate-700">{isZh ? "配置备份" : "Config Backup"}</p>
              <p className="text-xs text-slate-500">
                {isZh ? "导出或导入提供商、路由和设置" : "Export or import providers, routes & settings"}
              </p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                onClick={() => exportMut.mutate()}
                disabled={exportMut.isPending}
                size="sm"
                className="flex items-center gap-1.5"
              >
                {exportMut.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Download className="h-3.5 w-3.5" />
                )}
                {isZh ? "导出" : "Export"}
              </Button>
              <Button
                onClick={() => fileRef.current?.click()}
                disabled={importMut.isPending}
                variant="secondary"
                size="sm"
                className="flex items-center gap-1.5"
              >
                {importMut.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Upload className="h-3.5 w-3.5" />
                )}
                {isZh ? "导入" : "Import"}
              </Button>
              <input
                ref={fileRef}
                type="file"
                accept=".json"
                className="hidden"
                onChange={handleImportFile}
              />
            </div>
            {importMut.isSuccess && importMut.data && (
              <p className="text-xs text-green-600">
                {isZh
                  ? `已导入：${(importMut.data as ImportResult).providers_imported} 个提供商，${(importMut.data as ImportResult).routes_imported} 条路由，${(importMut.data as ImportResult).settings_imported} 项设置`
                  : `Imported: ${(importMut.data as ImportResult).providers_imported} providers, ${(importMut.data as ImportResult).routes_imported} routes, ${(importMut.data as ImportResult).settings_imported} settings`}
              </p>
            )}
          </div>

          <div className="rounded-xl bg-slate-50 p-4 space-y-3 md:col-span-2">
            <div>
              <p className="text-sm font-medium text-slate-700">{isZh ? "本地代理" : "Local Proxy"}</p>
              <p className="text-xs text-slate-500">
                {isZh ? "Provider 开启代理后会使用这里的代理地址发送请求" : "Providers with proxy enabled will route requests through this proxy"}
              </p>
            </div>
            <div className="flex items-center justify-between rounded-lg border border-slate-200 bg-white px-3 py-2.5">
              <span className="text-sm text-slate-700">{isZh ? "启用代理" : "Enable proxy"}</span>
              <Switch checked={proxyEnabled} onCheckedChange={setProxyEnabled} />
            </div>
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <div className="space-y-1.5">
                <label className="ml-1 text-xs text-slate-700">{isZh ? "代理 URL" : "Proxy URL"}</label>
                <Input
                  placeholder="http://127.0.0.1:7890"
                  value={proxyUrl}
                  onChange={(e) => setProxyUrl(e.target.value)}
                />
              </div>
              <div className="space-y-1.5">
                <label className="ml-1 text-xs text-slate-700">{isZh ? "绕过地址（可选）" : "Bypass hosts (optional)"}</label>
                <Input
                  placeholder={isZh ? "localhost,127.0.0.1,.internal" : "localhost,127.0.0.1,.internal"}
                  value={proxyBypass}
                  onChange={(e) => setProxyBypass(e.target.value)}
                />
              </div>
            </div>
            <div className="flex items-center gap-2">
              <Button
                onClick={() => saveProxySettings.mutate()}
                disabled={saveProxySettings.isPending}
                size="sm"
                className="flex items-center gap-1.5"
              >
                {saveProxySettings.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Save className="h-3.5 w-3.5" />
                )}
                {isZh ? "保存代理设置" : "Save Proxy Settings"}
              </Button>
              {saveProxySettings.isSuccess && (
                <p className="text-xs text-green-600">{isZh ? "代理设置已保存" : "Proxy settings saved"}</p>
              )}
            </div>
          </div>
        </div>
      </div>

      <ConfirmDialog
        open={Boolean(errorDialog)}
        onOpenChange={(open) => {
          if (!open) setErrorDialog(null);
        }}
        title={errorDialog?.title ?? ""}
        description={errorDialog?.description}
        hideCancel
        confirmText={isZh ? "我知道了" : "OK"}
        onConfirm={() => setErrorDialog(null)}
      />
    </div>
  );
}
