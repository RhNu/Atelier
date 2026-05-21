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
  label: string;
  icon: LucideIcon;
};

export const routeNavItems: ReadonlyArray<RouteNavItem> = [
  { to: "/generate", label: "Generate", icon: WandSparkles },
  { to: "/director", label: "Director", icon: Clapperboard },
  { to: "/resources", label: "Resources", icon: Boxes },
  { to: "/lexicon", label: "Lexicon", icon: BookOpen },
  { to: "/gallery", label: "Gallery", icon: Images },
  { to: "/settings", label: "Settings", icon: Settings },
];
