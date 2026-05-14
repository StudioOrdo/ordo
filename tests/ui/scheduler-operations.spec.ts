import { expect, test } from "@playwright/test";

import { formatScheduleTimestamp } from "@/lib/scheduler-operations";

test.describe("scheduler operations view model", () => {
  test("formats schedule timestamps in deterministic UTC", () => {
    expect(formatScheduleTimestamp("2026-05-07T10:00:00Z")).toBe("2026-05-07T10:00:00Z");
    expect(formatScheduleTimestamp("2026-05-07T10:00:00-04:00")).toBe("2026-05-07T14:00:00Z");
  });

  test("handles malformed timestamps without host locale output", () => {
    expect(formatScheduleTimestamp("not-a-date")).toBe("invalid timestamp");
  });
});
