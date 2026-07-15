import {
  BookOpen,
  Boxes,
  Clapperboard,
  Images,
  Settings,
  WandSparkles,
  type LucideIcon,
} from "lucide-react";

export type RouteNavItem = {
  to: "/generate" | "/director" | "/resources" | "/lexicon" | "/gallery" | "/settings";
  labelKey: `nav.${"generate" | "director" | "resources" | "lexicon" | "gallery" | "settings"}`;
  icon: LucideIcon;
};

export const routeNavItems: ReadonlyArray<RouteNavItem> = [
  { to: "/generate", labelKey: "nav.generate", icon: WandSparkles },
  { to: "/director", labelKey: "nav.director", icon: Clapperboard },
  { to: "/resources", labelKey: "nav.resources", icon: Boxes },
  { to: "/lexicon", labelKey: "nav.lexicon", icon: BookOpen },
  { to: "/gallery", labelKey: "nav.gallery", icon: Images },
  { to: "/settings", labelKey: "nav.settings", icon: Settings },
];
