import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Station } from "@/types/station";
import * as react from "react";
import { useNavigate } from "react-router-dom";

export interface StationCardProps extends react.HTMLAttributes<HTMLDivElement> {
  station: Station;
}

const getStyleFromStationStatus = (station: Station): string => {
  if (station.value === undefined) return "";
  if (station.soglia3 > 0 && station.value >= station.soglia3) {
    return "bg-red-500";
  }
  if (station.soglia2 > 0 && station.value >= station.soglia2) {
    return "orange-500";
  }
  if (station.soglia1 > 0 && station.value >= station.soglia1) {
    return "yellow-500";
  }
  return "green-500";
};

export const StationCard = react.forwardRef<HTMLDivElement, StationCardProps>(
  ({ station, className, ...props }, ref) => {
    const navigate = useNavigate();
    return (
      <Card
        className={cn(className, getStyleFromStationStatus(station))}
        {...props}
        ref={ref}
      >
        <CardHeader>
          <CardTitle>{station.nomestaz}</CardTitle>
        </CardHeader>
        <CardContent>
          <p>Utilma rilevazione: {station.value}</p>
          <p>Soglia1: {station.soglia1}m</p>
          <p>Soglia2: {station.soglia2}m</p>
          <p>Soglia3: {station.soglia3}m</p>
          <Button
            onClick={() =>
              navigate(`/timeseries`, { state: { station: station } })
            }
          >
            Next
          </Button>
        </CardContent>
      </Card>
    );
  },
);
