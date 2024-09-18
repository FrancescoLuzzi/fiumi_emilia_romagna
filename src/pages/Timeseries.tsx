import { useParams } from "react-router-dom";

type TimeriesParams = {
  station_id: string;
};

export const Timeseries = () => {
  const { station_id } = useParams<TimeriesParams>();
  return <div>{station_id}</div>;
};
