import { renderHook, act } from "@testing-library/react";
import { useTheme } from "./useTheme";

function mockMatchMedia(matchesDark: boolean) {
  window.matchMedia = (q: string) => ({
    matches: q.includes("dark") ? matchesDark : false,
    media: q, addEventListener(){}, removeEventListener(){},
    addListener(){}, removeListener(){}, onchange:null, dispatchEvent:()=>false,
  }) as unknown as MediaQueryList;
}

test("follows system dark and toggles", () => {
  localStorage.clear();
  mockMatchMedia(true);
  const { result } = renderHook(() => useTheme());
  expect(result.current.theme).toBe("dark");
  expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  act(() => result.current.toggle());
  expect(result.current.theme).toBe("light");
  expect(document.documentElement.getAttribute("data-theme")).toBe("light");
});
