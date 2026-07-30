import {
  BookOpen,
  Boxes,
  Clapperboard,
  Images,
  Settings,
  Telescope,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";

export type RouteNavItem = {
  to:
    | "/generate"
    | "/director"
    | "/resources"
    | "/lexicon"
    | "/inspiration"
    | "/gallery"
    | "/settings";
  labelKey: `nav.${"generate" | "director" | "resources" | "lexicon" | "inspiration" | "gallery" | "settings"}`;
  icon: LucideIcon;
};

export const primaryRouteNavItems: ReadonlyArray<RouteNavItem> = [
  { to: "/generate", labelKey: "nav.generate", icon: WandSparkles },
  { to: "/director", labelKey: "nav.director", icon: Clapperboard },
  { to: "/resources", labelKey: "nav.resources", icon: Boxes },
  { to: "/lexicon", labelKey: "nav.lexicon", icon: BookOpen },
  { to: "/inspiration", labelKey: "nav.inspiration", icon: Telescope },
  { to: "/gallery", labelKey: "nav.gallery", icon: Images },
];

export const settingsNavItem: RouteNavItem = {
  to: "/settings",
  labelKey: "nav.settings",
  icon: Settings,
};

export const routeNavItems: ReadonlyArray<RouteNavItem> = [
  ...primaryRouteNavItems,
  settingsNavItem,
];
