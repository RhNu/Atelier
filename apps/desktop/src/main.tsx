import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import "./index.css";
import App from "./App.tsx";
import { AtelierEventBridge } from "./app/AtelierEventBridge";
import { createAtelierQueryClient } from "./app/query-client";

const queryClient = createAtelierQueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <AtelierEventBridge />
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
