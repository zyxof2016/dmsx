import React from "react";
import {
  App as AntApp,
  Layout,
  Menu,
  Breadcrumb,
  Segmented,
  Space,
  Select,
  Button,
  Dropdown,
  Tag,
  Tooltip,
  Typography,
  ConfigProvider,
  theme as antdTheme,
} from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import {
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  ReloadOutlined,
  SearchOutlined,
  TranslationOutlined,
  BulbOutlined,
  MoonOutlined,
} from "@ant-design/icons";
import {
  Outlet,
  useNavigate,
  useRouterState,
} from "@tanstack/react-router";

import {
  useAppI18n,
  useThemeMode,
  useAppSession,
  type AppMode,
  type Lang,
} from "./appProviders";
import { AccessGate } from "./components/AccessGate";
import { useResourceAccess } from "./authz";
import {
  NAV_ITEMS,
  buildGroupedMenuItems,
  evaluateItemAccess,
  itemIsVisible,
  matchNavItem,
  navItemLabel,
  type NavItem,
} from "./navigation";

const AppDeferredTools = React.lazy(async () => {
  const mod = await import("./components/AppDeferredTools");
  return { default: mod.AppDeferredTools };
});

const { Header, Sider, Content } = Layout;
const { Text } = Typography;

export const AppLayout: React.FC = () => {
  const { lang } = useAppI18n();
  const { themeMode } = useThemeMode();
  const locale = lang === "zh" ? zhCN : enUS;
  const algorithm =
    themeMode === "dark" ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm;

  return (
    <ConfigProvider
      locale={locale}
      theme={{
        algorithm,
        token: {
          colorPrimary: "#2563eb",
          borderRadius: 6,
          colorBgContainer: themeMode === "dark" ? "#15171d" : "#ffffff",
          colorBgElevated: themeMode === "dark" ? "#1b1e26" : "#ffffff",
          colorBgLayout: themeMode === "dark" ? "#0f1117" : "#f3f6fb",
          colorBorderSecondary: themeMode === "dark" ? "#2a2f3b" : "#e5e7eb",
        },
        components: {
          Card: {
            borderRadiusLG: 8,
          },
          Modal: {
            borderRadiusLG: 8,
          },
          Menu: {
            itemBorderRadius: 6,
            itemMarginInline: 10,
          },
        },
      }}
    >
      <AntApp>
        <AppShell />
      </AntApp>
    </ConfigProvider>
  );
};

const AppShell: React.FC = () => {
  const [collapsed, setCollapsed] = React.useState(false);
  const [visitedKeys, setVisitedKeys] = React.useState<string[]>([]);
  const navigate = useNavigate();
  const pathname = useRouterState({
    select: (s) => s.location.pathname,
  });

  const selectedNavItem = matchNavItem(pathname);
  const selectedKey = selectedNavItem?.key ?? "dashboard";

  const { t, lang, setLang } = useAppI18n();
  const { themeMode, setThemeMode } = useThemeMode();
  const {
    tenantId,
    setTenantId,
    jwt,
    setJwt,
    clearJwt,
    appMode,
    setAppMode,
    effectiveRoles,
    canUsePlatformMode,
    platformRoles,
    subject,
    tenantOptions,
    authMode,
    isAuthenticated,
    displayName,
    availableScopes,
  } = useAppSession();
  const { token } = antdTheme.useToken();
  const platformReadAccess = useResourceAccess("platformRead");

  const visibleNavItems = React.useMemo(
    () =>
      NAV_ITEMS.filter((item) =>
        itemIsVisible(item, appMode, effectiveRoles, canUsePlatformMode),
      ),
    [appMode, canUsePlatformMode, effectiveRoles],
  );
  const breadcrumbLabel = navItemLabel(selectedNavItem, t);
  const modeLabel = t(`mode.${appMode}`);
  const defaultModePath = visibleNavItems[0]?.path ?? "/";
  const selectedAccess = evaluateItemAccess(
    selectedNavItem,
    appMode,
    effectiveRoles,
    canUsePlatformMode,
  );
  const activeRoles = appMode === "platform" ? platformRoles : effectiveRoles;
  const hasAccess =
    selectedAccess.allowed && (appMode !== "platform" || platformReadAccess.canRead);
  const showModeSwitcher = availableScopes.includes("platform") && availableScopes.includes("tenant");
  const showPageTitle = appMode !== "platform";
  const visitedNavItems = React.useMemo(
    () =>
      visitedKeys
        .map((key) => visibleNavItems.find((item) => item.key === key))
        .filter((item): item is NavItem => Boolean(item)),
    [visibleNavItems, visitedKeys],
  );
  const quickNavOptions = React.useMemo(
    () =>
      visibleNavItems.map((item) => ({
        value: item.key,
        label: `${t(item.groupKey)} / ${t(item.labelKey)}`,
      })),
    [t, visibleNavItems],
  );
  const groupedMenuItems = React.useMemo(
    () => buildGroupedMenuItems(visibleNavItems, t),
    [t, visibleNavItems],
  );

  const accessDescription =
    selectedAccess.reason === "mode"
      ? "当前页面属于另一种工作模式。请切换模式，或返回当前模式首页。"
      : selectedAccess.reason === "platform"
        ? "当前 JWT 不具备平台级角色，不能进入平台模式页面。"
        : selectedAccess.reason === "role"
          ? "当前角色不足以展示该页面入口，请检查 JWT 中的 roles / tenant_roles。"
          : "当前页面不可访问。";

  React.useEffect(() => {
    if (
      !selectedNavItem &&
      !visibleNavItems.some((item) => item.key === selectedKey) &&
      pathname !== defaultModePath
    ) {
      navigate({ to: defaultModePath, replace: true });
    }
  }, [defaultModePath, navigate, pathname, selectedKey, selectedNavItem, visibleNavItems]);

  React.useEffect(() => {
    if (!selectedNavItem) return;
    setVisitedKeys((prev) => {
      const next = [selectedNavItem.key, ...prev.filter((key) => key !== selectedNavItem.key)];
      return next.slice(0, 8);
    });
  }, [selectedNavItem]);

  React.useEffect(() => {
    if (pathname === "/" && appMode === "platform" && defaultModePath !== "/") {
      navigate({ to: defaultModePath, replace: true });
    }
  }, [appMode, defaultModePath, navigate, pathname]);

  React.useEffect(() => {
    if (pathname === "/login" || pathname === "/zero-touch-enroll") return;
    if (authMode === "jwt" && !isAuthenticated) {
      navigate({ to: "/login", search: { redirect: pathname }, replace: true });
    }
  }, [authMode, isAuthenticated, navigate, pathname]);

  if (pathname === "/login" || pathname === "/zero-touch-enroll") {
    return <Outlet />;
  }

  if (authMode === "jwt" && !isAuthenticated) {
    return null;
  }

  return (
    <Layout className={`dmsx-shell dmsx-shell-${themeMode}`}>
      <Sider
        className="dmsx-sider"
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        theme={themeMode === "dark" ? "dark" : "light"}
        trigger={null}
        width={232}
      >
        <div className="dmsx-brand">
          <div className="dmsx-brand-mark">DX</div>
          {!collapsed && (
            <div className="dmsx-brand-copy">
              <Text strong>{t("brand.full")}</Text>
              <span>{modeLabel}</span>
            </div>
          )}
        </div>
        <Menu
          className="dmsx-menu"
          theme={themeMode === "dark" ? "dark" : "light"}
          mode="inline"
          selectedKeys={[selectedKey]}
          items={groupedMenuItems}
          onClick={({ key }) => {
            const target = visibleNavItems.find((item) => item.key === key)?.path;
            if (target) navigate({ to: target });
          }}
        />
      </Sider>

      <Layout className="dmsx-main">
        <Header
          className="dmsx-header"
          style={{ background: token.colorBgElevated, borderBottom: `1px solid ${token.colorBorderSecondary}` }}
        >
          <Space size="middle" className="dmsx-header-left">
            <Tooltip title={collapsed ? "展开菜单" : "收起菜单"}>
              <Button
                type="text"
                icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                onClick={() => setCollapsed((value) => !value)}
              />
            </Tooltip>
            <Breadcrumb
              items={[
                { title: t("brand") },
                { title: modeLabel },
                { title: breadcrumbLabel },
              ]}
            />
            <Select
              className="dmsx-quick-nav"
              size="small"
              showSearch
              allowClear
              placeholder="搜索菜单"
              suffixIcon={<SearchOutlined />}
              value={undefined}
              options={quickNavOptions}
              optionFilterProp="label"
              onChange={(key) => {
                const target = visibleNavItems.find((item) => item.key === key)?.path;
                if (target) navigate({ to: target });
              }}
            />
            {showModeSwitcher && (
              <Segmented
                className="dmsx-mode-switch"
                size="small"
                value={appMode}
                options={[
                  { value: "tenant", label: t("mode.tenantShort") },
                  {
                    value: "platform",
                    label: t("mode.platformShort"),
                    disabled: !canUsePlatformMode,
                  },
                ]}
                onChange={(value) => {
                  const nextMode = value as AppMode;
                  setAppMode(nextMode);
                  const nextPath = NAV_ITEMS.find((item) =>
                    itemIsVisible(item, nextMode, effectiveRoles, canUsePlatformMode),
                  )?.path;
                  if (nextPath && !NAV_ITEMS.some((item) => item.key === selectedKey && item.path === nextPath)) {
                    navigate({ to: nextPath });
                  }
                }}
              />
            )}
          </Space>
          <Space size={8} className="dmsx-header-right">
            <Tooltip title="刷新当前页">
              <Button
                className="dmsx-tool-button"
                type="text"
                icon={<ReloadOutlined />}
                onClick={() => window.location.reload()}
              />
            </Tooltip>
            <Dropdown
              menu={{
                selectedKeys: [lang],
                items: [
                  { key: "zh", label: "中文" },
                  { key: "en", label: "English" },
                ],
                onClick: ({ key }) => setLang(key as Lang),
              }}
              trigger={["click"]}
            >
              <Tooltip title="语言">
                <Button className="dmsx-tool-button" type="text" icon={<TranslationOutlined />} />
              </Tooltip>
            </Dropdown>
            <Tooltip title={themeMode === "dark" ? "切换到亮色" : "切换到暗色"}>
              <Button
                className="dmsx-tool-button"
                type="text"
                icon={themeMode === "dark" ? <BulbOutlined /> : <MoonOutlined />}
                onClick={() => setThemeMode(themeMode === "dark" ? "light" : "dark")}
              />
            </Tooltip>
            <React.Suspense fallback={null}>
              <AppDeferredTools
                tenantId={tenantId}
                tenantOptions={tenantOptions}
                jwt={jwt}
                userLabel={displayName ?? subject ?? t("user.admin")}
                profileLabel={t("user.profile")}
                logoutLabel={t("user.logout")}
                aiTooltip={t("ai.assistant")}
                setTenantId={setTenantId}
                setJwt={setJwt}
                clearJwt={clearJwt}
                onLoggedOut={() => navigate({ to: "/login", replace: true })}
                showTenantShortcut={appMode === "tenant"}
                onOpenAi={() => navigate({ to: "/ai" })}
              />
            </React.Suspense>
          </Space>
        </Header>

        <div className="dmsx-tabsbar" style={{ background: token.colorBgElevated, borderBottom: `1px solid ${token.colorBorderSecondary}` }}>
          {visitedNavItems.map((item) => (
            <Tag
              key={item.key}
              closable={visitedNavItems.length > 1}
              color={item.key === selectedKey ? "blue" : "default"}
              onClick={() => navigate({ to: item.path })}
              onClose={(event) => {
                event.preventDefault();
                setVisitedKeys((prev) => {
                  const next = prev.filter((key) => key !== item.key);
                  if (item.key === selectedKey) {
                    const fallback = next
                      .map((key) => visibleNavItems.find((candidate) => candidate.key === key))
                      .find(Boolean);
                    navigate({ to: fallback?.path ?? defaultModePath });
                  }
                  return next;
                });
              }}
            >
              {t(item.labelKey)}
            </Tag>
          ))}
        </div>

        <Content className="dmsx-content">
          {showPageTitle && (
            <div className="dmsx-page-title">
              <div>
                <Text type="secondary">{modeLabel}</Text>
                <Typography.Title level={4}>{breadcrumbLabel}</Typography.Title>
              </div>
              <Space wrap size={6}>
                {activeRoles.slice(0, 4).map((role) => (
                  <Tag key={role}>{role}</Tag>
                ))}
              </Space>
            </div>
          )}
          <div
            className="dmsx-page-surface"
            style={{
              background: token.colorBgContainer,
              borderColor: token.colorBorderSecondary,
            }}
          >
            {!hasAccess ? (
              <AccessGate
                title="当前页面无访问权限"
                description={accessDescription}
                roles={activeRoles}
                modeLabel={modeLabel}
                onGoDefault={() => navigate({ to: defaultModePath })}
                onSwitchMode={
                  showModeSwitcher
                    ? () => {
                        const nextMode: AppMode = appMode === "tenant" ? "platform" : "tenant";
                        setAppMode(nextMode);
                        const nextPath = NAV_ITEMS.find((item) =>
                          itemIsVisible(item, nextMode, effectiveRoles, canUsePlatformMode),
                        )?.path;
                        navigate({ to: nextPath ?? "/" });
                      }
                    : undefined
                }
                switchModeLabel={showModeSwitcher ? (appMode === "tenant" ? "切换到平台模式" : "切换到租户模式") : undefined}
              />
            ) : (
              <Outlet />
            )}
          </div>
        </Content>
      </Layout>
    </Layout>
  );
};
