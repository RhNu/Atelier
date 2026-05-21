import { RouterProvider } from "@tanstack/react-router";

import { router } from "./routes/router";
import { AppErrorBoundary } from "./shell/AppErrorBoundary";

function App() {
  return (
    <AppErrorBoundary>
      <RouterProvider router={router} />
    </AppErrorBoundary>
  );
}

export default App;
