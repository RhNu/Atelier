import type { LocaleShape } from "../../locale-types";
import { promptEditor as enPromptEditor } from "../en/promptEditor";

export const promptEditor = {
  completions: "提示词补全",
  reusableChunk: "可复用提示词片段",
  compileTimeComment: "编译提示词时移除的注释",
  promptChunk: "提示词片段",
  tokenUsage: "已使用 {{used}} / {{limit}} Token",
  diagnostic: {
    unmatchedStrengtheningClose: "增强结束符没有对应的开始符。",
    unmatchedWeakeningClose: "减弱结束符没有对应的开始符。",
    unclosedStrengthening: "增强区块尚未闭合。",
    unclosedWeakening: "减弱区块尚未闭合。",
    invalidNumericWeight: "数值权重无效。",
    unsupportedNumericWeight: "当前模型不支持数值权重。",
    unsupportedNegativeNumericWeight: "负数权重需要 NAI Diffusion 4.5。",
    unclosedNumericWeight: "数值权重将持续到提示词末尾。",
    unterminatedString: "字符串尚未闭合。",
    unclosedRandomizer: "随机选项尚未闭合。",
    emptyRandomizerOption: "随机选项中包含空项。",
    unclosedFunctionCall: "扩展函数调用尚未闭合。",
    unknownFunction: "未知的提示词扩展函数。",
    invalidFunctionArity: "提示词扩展函数的参数数量无效。",
    invalidFunctionArgument: "提示词扩展函数包含无效的命名参数。",
  },
} satisfies LocaleShape<typeof enPromptEditor>;
