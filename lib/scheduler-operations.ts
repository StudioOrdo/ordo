export function formatScheduleTimestamp(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "invalid timestamp";
  }
  return date.toISOString().replace(".000Z", "Z");
}
