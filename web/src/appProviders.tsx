import React from "react";
import {
  clearStoredJwt,
  getStoredJwt,
  getStoredTenantId,
  setStoredJwt,
  setStoredTenantId,
} from "./api/client";

export type ThemeMode = "light" | "dark";
export type Lang = "zh" | "en";

type I18nContextValue = {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: (key: string) => string;
};

type ThemeContextValue = {
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
  toggleTheme: () => void;
};

type SessionContextValue = {
  tenantId: string;
  setTenantId: (tenantId: string) => void;
  jwt: string;
  setJwt: (jwt: string) => void;
  clearJwt: () => void;
};

const LANG_KEY = "dmsx_lang";
const THEME_KEY = "dmsx_theme";

const I18nContext = React.createContext<I18nContextValue | null>(null);
const ThemeContext = React.createContext<ThemeContextValue | null>(null);
const SessionContext = React.createContext<SessionContextValue | null>(null);

const dictionaries: Record<Lang, Record<string, string>> = {
  zh: {
    brand: "DMSX",
    "brand.full": "DMSX 集控",
    theme: "主题",
    "theme.dark": "暗色",
    "theme.light": "亮色",
    "nav.dashboard": "态势总览",
    "nav.devices": "设备管理",
    "nav.policies": "策略中心",
    "nav.commands": "远程命令",
    "nav.artifacts": "应用分发",
    "nav.compliance": "安全合规",
    "nav.network": "网络管控",
    "nav.ai": "AI 智慧中心",
    "nav.settings": "系统设置",
    "nav.policyEditor": "策略编辑器",
    "nav.auditLogs": "审计日志",
    "nav.usersRoles": "用户 / 角色管理",
    "user.profile": "个人中心",
    "user.logout": "退出登录",
    "user.admin": "管理员",
    "ai.assistant": "AI 助手",
    "page.dashboard": "态势总览",
    "page.systemSettings": "系统设置",
    "page.policyEditor": "策略编辑器",
    "page.auditLogs": "审计日志",
    "page.usersRoles": "用户 / 角色管理",
    "common.loadFailed": "加载失败",
    "common.backendNotImplemented":
      "后端尚未提供该功能的 HTTP API，本页仅提供前端 UI / 校验 / 导出框架。",
    "buttons.refresh": "刷新",
    "buttons.copy": "复制",
    "buttons.saveDisabled": "保存（后端未接入）",
  },
  en: {
    brand: "DMSX",
    "brand.full": "DMSX Control Panel",
    theme: "Theme",
    "theme.dark": "Dark",
    "theme.light": "Light",
    "nav.dashboard": "Dashboard",
    "nav.devices": "Devices",
    "nav.policies": "Policies",
    "nav.commands": "Remote Commands",
    "nav.artifacts": "Artifacts",
    "nav.compliance": "Compliance",
    "nav.network": "Network",
    "nav.ai": "AI Center",
    "nav.settings": "System Settings",
    "nav.policyEditor": "Policy Editor",
    "nav.auditLogs": "Audit Logs",
    "nav.usersRoles": "Users / Roles",
    "user.profile": "Profile",
    "user.logout": "Logout",
    "user.admin": "Admin",
    "ai.assistant": "AI Assistant",
    "page.dashboard": "Dashboard",
    "page.systemSettings": "System Settings",
    "page.policyEditor": "Policy Editor",
    "page.auditLogs": "Audit Logs",
    "page.usersRoles": "Users / Roles",
    "common.loadFailed": "Load failed",
    "common.backendNotImplemented":
      "Backend HTTP API is not available yet. This page only provides UI/validation/export scaffolding.",
    "buttons.refresh": "Refresh",
    "buttons.copy": "Copy",
    "buttons.saveDisabled": "Save (backend not connected)",
  },
};

function getInitialLang(): Lang {
  const raw = localStorage.getItem(LANG_KEY);
  if (raw === "en" || raw === "zh") return raw;
  // Default: Chinese UI unless user explicitly prefers English.
  return "zh";
}

function getInitialTheme(): ThemeMode {
  const raw = localStorage.getItem(THEME_KEY);
  if (raw === "dark" || raw === "light") return raw;
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches
    ? "dark"
    : "light";
}

export function useAppI18n() {
  const ctx = React.useContext(I18nContext);
  if (!ctx) throw new Error("useAppI18n must be used within AppProviders");
  return ctx;
}

export function useThemeMode() {
  const ctx = React.useContext(ThemeContext);
  if (!ctx) throw new Error("useThemeMode must be used within AppProviders");
  return ctx;
}

export function useAppSession() {
  const ctx = React.useContext(SessionContext);
  if (!ctx) throw new Error("useAppSession must be used within AppProviders");
  return ctx;
}

export const AppProviders: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [lang, setLang] = React.useState<Lang>(() => getInitialLang());
  const [themeMode, setThemeMode] = React.useState<ThemeMode>(() =>
    getInitialTheme(),
  );
  const [tenantId, setTenantIdState] = React.useState<string>(() =>
    getStoredTenantId(),
  );
  const [jwt, setJwtState] = React.useState<string>(() => getStoredJwt() ?? "");

  React.useEffect(() => {
    localStorage.setItem(LANG_KEY, lang);
  }, [lang]);

  React.useEffect(() => {
    localStorage.setItem(THEME_KEY, themeMode);
  }, [themeMode]);

  const t = React.useCallback(
    (key: string) => {
      return dictionaries[lang][key] ?? key;
    },
    [lang],
  );

  const i18nValue = React.useMemo<I18nContextValue>(
    () => ({ lang, setLang, t }),
    [lang, setLang, t],
  );

  const themeValue = React.useMemo<ThemeContextValue>(
    () => ({
      themeMode,
      setThemeMode,
      toggleTheme: () =>
        setThemeMode((prev) => (prev === "dark" ? "light" : "dark")),
    }),
    [themeMode],
  );

  const sessionValue = React.useMemo<SessionContextValue>(
    () => ({
      tenantId,
      setTenantId: (nextTenantId: string) => {
        const value = nextTenantId.trim();
        setStoredTenantId(value);
        setTenantIdState(value);
      },
      jwt,
      setJwt: (nextJwt: string) => {
        const value = nextJwt.trim();
        setStoredJwt(value);
        setJwtState(value);
      },
      clearJwt: () => {
        clearStoredJwt();
        setJwtState("");
      },
    }),
    [jwt, tenantId],
  );

  return (
    <ThemeContext.Provider value={themeValue}>
      <I18nContext.Provider value={i18nValue}>
        <SessionContext.Provider value={sessionValue}>{children}</SessionContext.Provider>
      </I18nContext.Provider>
    </ThemeContext.Provider>
  );
};
