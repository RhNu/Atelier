import { QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./index.css";
import App from "./App.tsx";
import { AtelierEventBridge } from "./app/AtelierEventBridge";
import { createAtelierQueryClient } from "./app/query-client";

const queryClient = createAtelierQueryClient();
const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Atelier root element is missing.");
}

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <AtelierEventBridge />
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
