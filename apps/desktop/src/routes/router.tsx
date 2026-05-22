import { createRootRoute, createRoute, createRouter, redirect } from "@tanstack/react-router";

import { EmptyState } from "../components/ui";
import { DirectorPage } from "../features/director";
import { GalleryPage } from "../features/gallery";
import { GeneratePage } from "../features/generation";
import { LexiconPage } from "../features/lexicon";
import { ResourcesPage } from "../features/resources";
import { SettingsPage } from "../features/settings";
import { RootWorkbenchLayout } from "./RootWorkbenchLayout";
export { routeNavItems } from "./nav";

const rootRoute = createRootRoute({
  component: RootWorkbenchLayout,
  notFoundComponent: () => <EmptyState title="View not found" />,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/generate" });
  },
});

const generateRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/generate",
  component: GeneratePage,
});

const directorRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/director",
  component: DirectorPage,
});

const resourcesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/resources",
  component: ResourcesPage,
});

const lexiconRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/lexicon",
  component: LexiconPage,
});

const galleryRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/gallery",
  component: GalleryPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
});

export const appRouteTree = rootRoute.addChildren([
  indexRoute,
  generateRoute,
  directorRoute,
  resourcesRoute,
  lexiconRoute,
  galleryRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree: appRouteTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
