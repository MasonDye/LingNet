import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "@appica/ui-react/providers/theme-provider";
import App from "./App";
import { installLockdown } from "./lib/lockdown";
import "./index.css";

installLockdown();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="system">
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);
