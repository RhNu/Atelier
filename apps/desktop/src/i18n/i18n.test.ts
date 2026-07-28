import { afterEach, describe, expect, it } from "vitest";

import { applyLanguagePreference, i18n, resolveLanguagePreference, resolveSystemLanguage } from ".";
import { en, zhCN } from "./resources";

type TranslationTree = { readonly [key: string]: string | TranslationTree };

function collectKeys(value: TranslationTree, prefix = ""): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof child === "object" && child !== null ? collectKeys(child, path) : [path];
  });
}

afterEach(async () => {
  await applyLanguagePreference("en");
});

describe("frontend i18n", () => {
  it("keeps English and Simplified Chinese resources structurally aligned", () => {
    expect(collectKeys(zhCN)).toEqual(collectKeys(en));
  });

  it("maps any Chinese system locale to Simplified Chinese", () => {
    expect(resolveSystemLanguage(["zh-HK", "en-US"])).toBe("zh-CN");
    expect(resolveSystemLanguage(["ja-JP", "en-US"])).toBe("en");
  });

  it("resolves fixed language preferences without detection", () => {
    expect(resolveLanguagePreference("en")).toBe("en");
    expect(resolveLanguagePreference("zh-CN")).toBe("zh-CN");
  });

  it("changes the runtime language and document language together", async () => {
    await applyLanguagePreference("zh-CN");

    expect(i18n.resolvedLanguage).toBe("zh-CN");
    expect(document.documentElement.lang).toBe("zh-CN");
    expect(i18n.t("shell:openWorkspace")).toBe("打开工作区");
  });

  it("uses the agreed Simplified Chinese workflow terminology", () => {
    expect(zhCN.shell.nav.director).toBe("导演工具");
    expect(zhCN.generation.positive).toBe("提示词");
    expect(zhCN.generation.undesiredContent).toBe("负面内容");
    expect(zhCN.generation.ucPreset).toBe("负面提示预设");
    expect(zhCN.generation.ucPresetOptions).toEqual({
      heavy: "重度",
      light: "轻度",
      furry_focus: "兽类优先",
      human_focus: "人类优先",
      none: "无",
    });
    expect(zhCN.resources.promptChunks).toBe("提示词片段");
    expect(zhCN.director.tool.colorize.label).toBe("上色");
  });
});
