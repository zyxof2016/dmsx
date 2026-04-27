import React from "react";
import { useQuery } from "@tanstack/react-query";
import {
  clearStoredJwt,
  AUTH_EXPIRED_EVENT,
  api,
  getLastStoredTenantId,
  getStoredJwt,
  getStoredTenantId,
  setStoredJwt,
  setStoredTenantId,
  tenantPathFor,
} from "./api/client";
import type { TenantRbacMeResponse } from "./api/types";

export type ThemeMode = "light" | "dark";
export type Lang = "zh" | "en";
export type AppMode = "tenant" | "platform";

export type AuthMode = "disabled" | "jwt";

type ParsedJwtClaims = {
  subject: string | null;
  primaryTenantId: string | null;
  permittedTenantIds: string[];
  roles: string[];
  tenantRoles: Record<string, string[]>;
};

export type TenantOption = {
  id: string;
  name?: string | null;
  source: "jwt" | "recent";
  effectiveRoles: string[];
};

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
  appMode: AppMode;
  setAppMode: (mode: AppMode) => void;
  subject: string | null;
  primaryTenantId: string | null;
  globalRoles: string[];
  tenantRoles: Record<string, string[]>;
  permittedTenantIds: string[];
  tenantOptions: TenantOption[];
  effectiveRoles: string[];
  hasJwt: boolean;
  jwtParseError: boolean;
  canUsePlatformMode: boolean;
  isPlatformAdmin: boolean;
  platformRoles: string[];
  canWritePlatform: boolean;
  authMode: AuthMode;
  setAuthMode: (mode: AuthMode) => void;
  isAuthenticated: boolean;
  displayName: string | null;
  setDisplayName: (displayName: string | null) => void;
  availableScopes: AppMode[];
  setAvailableScopes: (scopes: AppMode[]) => void;
};

const LANG_KEY = "dmsx_lang";
const THEME_KEY = "dmsx_theme";
const MODE_KEY = "dmsx.mode";
const AUTH_MODE_KEY = "dmsx.auth_mode";
const RECENT_TENANTS_KEY = "dmsx.platform.recent_tenants";
const DISPLAY_NAME_KEY = "dmsx.display_name";
const AVAILABLE_SCOPES_KEY = "dmsx.available_scopes";

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
    "nav.platformTenants": "平台租户目录",
    "nav.platformQuotas": "平台配额",
    "nav.platformAudit": "全局审计",
    "nav.platformHealth": "平台健康",
    "nav.usersRoles": "用户 / 角色管理",
    "mode.platform": "平台管理",
    "mode.tenant": "租户管理",
    "mode.platformShort": "平台",
    "mode.tenantShort": "租户",
    "mode.switch": "工作模式",
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
    "nav.platformTenants": "Platform Tenants",
    "nav.platformQuotas": "Platform Quotas",
    "nav.platformAudit": "Global Audit",
    "nav.platformHealth": "Platform Health",
    "nav.usersRoles": "Users / Roles",
    "mode.platform": "Platform",
    "mode.tenant": "Tenant",
    "mode.platformShort": "Platform",
    "mode.tenantShort": "Tenant",
    "mode.switch": "Mode",
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

function getInitialMode(): AppMode {
  return localStorage.getItem(MODE_KEY) === "platform" ? "platform" : "tenant";
}

function getInitialAuthMode(): AuthMode {
  return localStorage.getItem(AUTH_MODE_KEY) === "disabled" ? "disabled" : "jwt";
}

function getInitialDisplayName(): string | null {
  const raw = localStorage.getItem(DISPLAY_NAME_KEY)?.trim();
  return raw || null;
}

function getInitialAvailableScopes(): AppMode[] {
  try {
    const raw = localStorage.getItem(AVAILABLE_SCOPES_KEY);
    if (!raw) return ["tenant", "platform"];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return ["tenant", "platform"];
    const scopes = parsed.filter((value): value is AppMode => value === "tenant" || value === "platform");
    return scopes.length ? Array.from(new Set(scopes)) : ["tenant", "platform"];
  } catch {
    return ["tenant", "platform"];
  }
}

function isValidUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
    value,
  );
}

function decodeBase64Url(value: string): string {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  const binary = window.atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

function uniqueStrings(values: string[]): string[] {
  return Array.from(new Set(values));
}

function getRecentTenantIds(): Array<{ id: string; name?: string | null }> {
  try {
    const raw = localStorage.getItem(RECENT_TENANTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Array<{ id?: unknown; name?: unknown }>;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((item) => ({
        id: typeof item?.id === "string" ? item.id : "",
        name: typeof item?.name === "string" ? item.name : null,
      }))
      .filter((item) => isValidUuid(item.id));
  } catch {
    return [];
  }
}

function parseJwtClaims(jwt: string): ParsedJwtClaims | null {
  const raw = jwt.trim();
  if (!raw) return null;

  const token = raw.startsWith("Bearer ") ? raw.slice(7).trim() : raw;
  const parts = token.split(".");
  if (parts.length < 2) return null;

  try {
    const payload = JSON.parse(decodeBase64Url(parts[1])) as Record<string, unknown>;
    const primaryTenantId =
      typeof payload.tenant_id === "string" && isValidUuid(payload.tenant_id)
        ? payload.tenant_id
        : null;
    const allowedTenantIds = asStringArray(payload.allowed_tenant_ids).filter(isValidUuid);
    const tenantRolesRaw =
      payload.tenant_roles && typeof payload.tenant_roles === "object"
        ? (payload.tenant_roles as Record<string, unknown>)
        : {};

    const tenantRoles = Object.fromEntries(
      Object.entries(tenantRolesRaw)
        .filter(([tenant]) => isValidUuid(tenant))
        .map(([tenant, roles]) => [tenant, uniqueStrings(asStringArray(roles))]),
    );

    return {
      subject: typeof payload.sub === "string" ? payload.sub : null,
      primaryTenantId,
      permittedTenantIds: uniqueStrings(
        [primaryTenantId, ...allowedTenantIds].filter((value): value is string => !!value),
      ),
      roles: uniqueStrings(asStringArray(payload.roles)),
      tenantRoles,
    };
  } catch {
    return null;
  }
}

function getEffectiveRolesForTenant(
  claims: ParsedJwtClaims | null,
  tenantId: string,
): string[] {
  if (!claims) return [];
  return claims.tenantRoles[tenantId] ?? claims.roles;
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
  const [appMode, setAppModeState] = React.useState<AppMode>(() => getInitialMode());
  const [authMode, setAuthModeState] = React.useState<AuthMode>(() => getInitialAuthMode());
  const [tenantId, setTenantIdState] = React.useState<string>(() =>
    getLastStoredTenantId() ?? getStoredTenantId(),
  );
  const [jwt, setJwtState] = React.useState<string>(() => getStoredJwt() ?? "");
  const [displayName, setDisplayNameState] = React.useState<string | null>(() => getInitialDisplayName());
  const [availableScopes, setAvailableScopesState] = React.useState<AppMode[]>(() => getInitialAvailableScopes());
  const jwtClaims = React.useMemo(() => parseJwtClaims(jwt), [jwt]);
  const hasJwt = jwt.trim().length > 0;
  const jwtParseError = hasJwt && !jwtClaims;
  const isAuthenticated = authMode === "disabled" || hasJwt;
  const tenantRbacMeQuery = useQuery({
    queryKey: ["tenantRbacMe", tenantId, jwtClaims?.subject ?? "anonymous", jwt],
    queryFn: () => api.get<TenantRbacMeResponse>(tenantPathFor(tenantId, "/rbac/me")),
    retry: false,
    enabled: authMode === "jwt" && hasJwt && !jwtParseError,
  });

  const permittedTenantIds = React.useMemo(
    () =>
      jwtClaims?.permittedTenantIds.length
        ? jwtClaims.permittedTenantIds
        : [tenantId],
    [jwtClaims, tenantId],
  );
  const tenantOptions = React.useMemo(() => {
    const recentById = new Map(getRecentTenantIds().map((item) => [item.id, item]));
    const ids = uniqueStrings([...permittedTenantIds, ...recentById.keys()]);

    return ids.map((id) => ({
      id,
      name: recentById.get(id)?.name ?? null,
      source: permittedTenantIds.includes(id) ? "jwt" : "recent",
      effectiveRoles: jwtClaims ? getEffectiveRolesForTenant(jwtClaims, id) : ["TenantAdmin"],
    } satisfies TenantOption));
  }, [jwtClaims, permittedTenantIds]);
  const effectiveRoles = React.useMemo(
    () =>
      tenantRbacMeQuery.data?.effective_roles?.length
        ? tenantRbacMeQuery.data.effective_roles
        :
      jwtClaims
        ? getEffectiveRolesForTenant(jwtClaims, tenantId)
        : appMode === "platform"
          ? ["PlatformAdmin"]
          : ["TenantAdmin"],
    [appMode, jwtClaims, tenantId, tenantRbacMeQuery.data?.effective_roles],
  );
  const platformRoles = React.useMemo(() => {
    if (!jwtClaims) return ["PlatformAdmin"];
    return jwtClaims.roles;
  }, [jwtClaims]);
  const canUsePlatformMode = availableScopes.includes("platform") && platformRoles.some((role) => ["PlatformAdmin", "PlatformViewer"].includes(role));
  const isPlatformAdmin = platformRoles.includes("PlatformAdmin");
  const canWritePlatform = isPlatformAdmin;
  const canUseTenantMode = availableScopes.includes("tenant") && permittedTenantIds.length > 0;

  React.useEffect(() => {
    localStorage.setItem(LANG_KEY, lang);
  }, [lang]);

  React.useEffect(() => {
    localStorage.setItem(THEME_KEY, themeMode);
  }, [themeMode]);

  React.useEffect(() => {
    if (jwtClaims?.primaryTenantId && !permittedTenantIds.includes(tenantId)) {
      setStoredTenantId(jwtClaims.primaryTenantId);
      setTenantIdState(jwtClaims.primaryTenantId);
    }
  }, [jwtClaims, permittedTenantIds, tenantId]);

  React.useEffect(() => {
    if (!canUsePlatformMode && appMode === "platform") {
      localStorage.setItem(MODE_KEY, "tenant");
      setAppModeState("tenant");
    }
    if (!canUseTenantMode && appMode === "tenant") {
      localStorage.setItem(MODE_KEY, "platform");
      setAppModeState("platform");
    }
  }, [appMode, canUsePlatformMode, canUseTenantMode]);

  React.useEffect(() => {
    localStorage.setItem(AUTH_MODE_KEY, authMode);
  }, [authMode]);

  React.useEffect(() => {
    if (displayName) {
      localStorage.setItem(DISPLAY_NAME_KEY, displayName);
    } else {
      localStorage.removeItem(DISPLAY_NAME_KEY);
    }
  }, [displayName]);

  React.useEffect(() => {
    localStorage.setItem(AVAILABLE_SCOPES_KEY, JSON.stringify(availableScopes));
  }, [availableScopes]);

  React.useEffect(() => {
    const onAuthExpired = () => {
      clearStoredJwt();
      setJwtState("");
      setDisplayNameState(null);
      setAvailableScopesState(["tenant", "platform"]);
    };
    window.addEventListener(AUTH_EXPIRED_EVENT, onAuthExpired);
    return () => window.removeEventListener(AUTH_EXPIRED_EVENT, onAuthExpired);
  }, []);

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
        setDisplayNameState(null);
        setAvailableScopesState(["tenant", "platform"]);
      },
      appMode,
      setAppMode: (mode: AppMode) => {
        localStorage.setItem(MODE_KEY, mode);
        setAppModeState(mode);
      },
      subject: jwtClaims?.subject ?? null,
      primaryTenantId: jwtClaims?.primaryTenantId ?? null,
      globalRoles: jwtClaims?.roles ?? [],
      tenantRoles: jwtClaims?.tenantRoles ?? {},
      permittedTenantIds,
      tenantOptions,
      effectiveRoles,
      hasJwt,
      jwtParseError,
      canUsePlatformMode,
      isPlatformAdmin,
      platformRoles,
      canWritePlatform,
      authMode,
      setAuthMode: (mode: AuthMode) => {
        localStorage.setItem(AUTH_MODE_KEY, mode);
        setAuthModeState(mode);
      },
      isAuthenticated,
      displayName,
      setDisplayName: (nextDisplayName: string | null) => {
        const value = nextDisplayName?.trim() ?? "";
        setDisplayNameState(value || null);
      },
      availableScopes,
      setAvailableScopes: (scopes: AppMode[]) => {
        const next = Array.from(new Set(scopes.filter((scope): scope is AppMode => scope === "tenant" || scope === "platform")));
        setAvailableScopesState(next.length ? next : ["tenant"]);
      },
    }),
    [
      appMode,
      canUsePlatformMode,
      effectiveRoles,
      authMode,
      hasJwt,
      isAuthenticated,
      isPlatformAdmin,
      jwt,
      jwtClaims?.subject,
      jwtClaims?.primaryTenantId,
      jwtClaims?.roles,
      jwtClaims?.tenantRoles,
      jwtParseError,
      platformRoles,
      permittedTenantIds,
      tenantOptions,
      tenantId,
      canWritePlatform,
      displayName,
      availableScopes,
    ],
  );

  return (
    <ThemeContext.Provider value={themeValue}>
      <I18nContext.Provider value={i18nValue}>
        <SessionContext.Provider value={sessionValue}>{children}</SessionContext.Provider>
      </I18nContext.Provider>
    </ThemeContext.Provider>
  );
};
