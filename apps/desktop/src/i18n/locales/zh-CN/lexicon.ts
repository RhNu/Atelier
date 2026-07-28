import type { LocaleShape } from "../../locale-types";
import { lexicon as enLexicon } from "../en/lexicon";

export const lexicon = {
  views: "词典视图",
  range: "第 {{from}}-{{to}} 项，共 {{total}} 项",
  weight: "权重 {{value}}",
  matched: "匹配：{{value}}",
  uncategorized: "未分类",
  catalog: "目录",
  search: "搜索",
  allTags: "全部标签",
  searchTitle: "搜索：{{query}}",
  searchResults: "搜索结果",
  enterSearch: "请输入搜索内容或尝试其他关键词",
  noMatchingTags: "没有匹配的标签",
  searchTags: "搜索标签",
  searchPlaceholder: "标签、翻译或别名",
  loadingCatalog: "正在加载词典目录",
  unavailable: "词典不可用",
  tags: "标签",
  translations: "翻译",
  categories: "词典分类",
  previousPage: "词典上一页",
  nextPage: "词典下一页",
  loadingEntries: "正在加载词典条目",
  queryFailed: "词典查询失败",
} satisfies LocaleShape<typeof enLexicon>;
