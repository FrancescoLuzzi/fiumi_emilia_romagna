import { getStationsNow } from "@/api/stations";
import { StationCard } from "@/components/station-card";
import { Stations } from "@/types/station";
import { useEffect, useState } from "react";
export const Home = () => {
  const [stations, setStations] = useState<Stations>([]);
  const [error, setError] = useState<string>("");
  useEffect(() => {
    getStationsNow().then(setStations).catch(setError);
  }, []);
  return (
    <div>
      <div>ERROR: {error}</div>
      <ul>
        {stations.map((x) => (
          <li>
            <StationCard station={x} />
          </li>
        ))}
      </ul>
    </div>
  );
};
