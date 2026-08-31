import { useCallback, useState } from "react";

import type { NovelAiExploreSortDto, NovelAiExplorePeriodDto } from "@/types";

export function useNovelAiExplorePreferences() {
  const [sort, setSortValue] = useState<NovelAiExploreSortDto>(() => {
    const stored = window.localStorage.getItem("atelier.explore.novelai.sort.v1");
    return stored === "top" || stored === "hot" || stored === "random" ? stored : "new";
  });
  const setSort = useCallback((value: NovelAiExploreSortDto) => {
    setSortValue(value);
    window.localStorage.setItem("atelier.explore.novelai.sort.v1", value);
  }, []);
  const [period, setPeriodValue] = useState<NovelAiExplorePeriodDto>(() => {
    const stored = window.localStorage.getItem("atelier.explore.novelai.period.v1");
    return stored === "day" || stored === "month" ? stored : "week";
  });
  const setPeriod = useCallback((value: NovelAiExplorePeriodDto) => {
    setPeriodValue(value);
    window.localStorage.setItem("atelier.explore.novelai.period.v1", value);
  }, []);
  return { sort, setSort, period, setPeriod };
}
