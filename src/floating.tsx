import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider, App as AntdApp, theme } from "antd";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { I18nextProvider } from "react-i18next";
import i18n from "@/i18n";
import { FloatingWindow } from "@/pages/FloatingWindow";
import "@/index.css";

const locale = i18n.language.startsWith("zh") ? zhCN : enUS;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <I18nextProvider i18n={i18n}>
      <ConfigProvider
        locale={locale}
        theme={{
          algorithm: theme.darkAlgorithm,
          token: {
            colorPrimary: "#38bdf8",
            colorBgBase: "#0F131C",
            borderRadius: 8,
          },
        }}
      >
        <AntdApp>
          <FloatingWindow />
        </AntdApp>
      </ConfigProvider>
    </I18nextProvider>
  </React.StrictMode>,
);
