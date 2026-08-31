import { createRootRoute, createRoute, createRouter, redirect } from "@tanstack/react-router";

import { DirectorPage } from "../features/director";
import { ExplorePage } from "../features/explore";
import { GalleryPage } from "../features/gallery";
import { GeneratePage } from "../features/generation";
import { LexiconPage } from "../features/lexicon";
import { ResourcesPage } from "../features/resources";
import { SettingsPage } from "../features/settings";
import { NotFoundView } from "./NotFoundView";
import { RootWorkbenchLayout } from "./RootWorkbenchLayout";
export { routeNavItems } from "./nav";

const rootRoute = createRootRoute({
  component: RootWorkbenchLayout,
  notFoundComponent: NotFoundView,
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

const exploreRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/explore",
  component: ExplorePage,
});

const legacyInspirationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/inspiration",
  beforeLoad: () => {
    throw redirect({ to: "/explore" });
  },
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
  exploreRoute,
  legacyInspirationRoute,
  galleryRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree: appRouteTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
