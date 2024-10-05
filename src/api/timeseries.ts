import { Station } from "@/types/station";
import { TimeSeries } from "@/types/timeseries";
import { invoke } from "@tauri-apps/api/core";

export async function getTimeSeries(station: Station): Promise<TimeSeries> {
  return await invoke("get_time_series", { station: station });
}
