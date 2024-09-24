import { Stations } from "@/types/station";
import { invoke } from "@tauri-apps/api/core";

export async function getStationsNow(): Promise<Stations> {
  return await invoke("get_stations_now");
}

export async function getStations(date: Date): Promise<Stations> {
  return await invoke("get_stations", { date: date.toISOString() });
}
