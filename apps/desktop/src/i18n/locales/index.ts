import { common as enCommon } from "./en/common";
import { director as enDirector } from "./en/director";
import { gallery as enGallery } from "./en/gallery";
import { generation as enGeneration } from "./en/generation";
import { lexicon as enLexicon } from "./en/lexicon";
import { promptEditor as enPromptEditor } from "./en/promptEditor";
import { resources as enResources } from "./en/resources";
import { settings as enSettings } from "./en/settings";
import { shell as enShell } from "./en/shell";
import { common as zhCommon } from "./zh-CN/common";
import { director as zhDirector } from "./zh-CN/director";
import { gallery as zhGallery } from "./zh-CN/gallery";
import { generation as zhGeneration } from "./zh-CN/generation";
import { lexicon as zhLexicon } from "./zh-CN/lexicon";
import { promptEditor as zhPromptEditor } from "./zh-CN/promptEditor";
import { resources as zhResources } from "./zh-CN/resources";
import { settings as zhSettings } from "./zh-CN/settings";
import { shell as zhShell } from "./zh-CN/shell";

export const en = {
  common: enCommon,
  promptEditor: enPromptEditor,
  shell: enShell,
  settings: enSettings,
  generation: enGeneration,
  gallery: enGallery,
  resources: enResources,
  lexicon: enLexicon,
  director: enDirector,
} as const;

export const zhCN = {
  common: zhCommon,
  promptEditor: zhPromptEditor,
  shell: zhShell,
  settings: zhSettings,
  generation: zhGeneration,
  gallery: zhGallery,
  resources: zhResources,
  lexicon: zhLexicon,
  director: zhDirector,
} as const;

export const resources = { en, "zh-CN": zhCN } as const;
