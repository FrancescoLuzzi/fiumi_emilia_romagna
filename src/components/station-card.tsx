import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Station } from "@/types/station";
import * as react from "react";

export interface StationCardProps {
  station: Station;
}

export const StationCard = react.forwardRef<HTMLDivElement, StationCardProps>(
  ({ station, ...props }, ref) => {
    return (
      <Card {...props} ref={ref}>
        <CardHeader>
          <CardTitle>{station.nomestaz}</CardTitle>
          <CardDescription>Description</CardDescription>
        </CardHeader>
        <CardContent>
          Content
          <Button>Next</Button>
        </CardContent>
      </Card>
    );
  },
);
