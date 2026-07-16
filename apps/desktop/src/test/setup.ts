import "@testing-library/jest-dom/vitest";
import "@/i18n";

if (!Range.prototype.getClientRects) {
  const emptyClientRects: DOMRectList = Object.assign([], { item: () => null });
  Range.prototype.getClientRects = () => emptyClientRects;
}
if (!Range.prototype.getBoundingClientRect) {
  Range.prototype.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
}
