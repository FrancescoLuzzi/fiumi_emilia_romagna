"use client";

import { Link, useLocation } from "react-router-dom";
import { getTimeSeries } from "@/api/timeseries";
import { useEffect, useState } from "react";
import {
  TimeSeries,
  TimeValue,
  TimeSeriesWithThresholds,
  TimeValueWithThresholds,
} from "@/types/timeseries";
import { Station, getSoglia1, getSoglia2, getSoglia3 } from "@/types/station";
import { toast } from "sonner";
import { TrendingUp } from "lucide-react";
import { CartesianGrid, Line, LineChart, XAxis } from "recharts";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";

export const Timeseries = () => {
  const location = useLocation();
  const station: Station = location.state.station;
  const [timeSeries, setTimeSeries] = useState<TimeSeriesWithThresholds>([]);
  useEffect(() => {
    let lastPoint: TimeValue = { t: 0, v: 0 };
    const soglia1 = getSoglia1(station);
    const soglia2 = getSoglia2(station);
    const soglia3 = getSoglia3(station);
    getTimeSeries(station)
      .then((timeseries: TimeSeries) => {
        let tsWithThresholds: TimeSeriesWithThresholds = timeseries.map(
          (point): TimeValueWithThresholds => {
            point.v = point.v || lastPoint.v;
            lastPoint.v = point.v;
            return {
              time: point.t,
              value: point.v!,
              soglia1,
              soglia2,
              soglia3,
            };
          },
        );
        setTimeSeries(tsWithThresholds);
      })
      .catch(toast.error);
  }, [station]);
  return (
    <div>
      <Link to="/">GO BACK</Link>
      <TimeseriesLineChart timeseries={timeSeries} />;
    </div>
  );
};

const chartConfig = {
  value: {
    label: "Rilevazione",
    color: "hsl(var(--chart-3))",
  },
  soglia1: {
    label: "Soglia1",
    color: "hsl(var(--chart-2))",
  },
  soglia2: {
    label: "Soglia2",
    color: "hsl(var(--chart-4))",
  },
  soglia3: {
    label: "Soglia3",
    color: "hsl(var(--chart-1))",
  },
} satisfies ChartConfig;

export function TimeseriesLineChart({
  timeseries,
}: {
  timeseries: TimeSeriesWithThresholds;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Rilevazioni</CardTitle>
        <CardDescription>January - June 2024</CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig}>
          <LineChart
            accessibilityLayer
            data={timeseries}
            margin={{
              left: 12,
              right: 12,
            }}
          >
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="time"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              // tickFormatter={(value) => value.slice(0, 3)}
            />
            <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
            <Line
              dataKey="value"
              type="monotone"
              stroke="var(--color-value)"
              strokeWidth={2}
              dot={false}
            />
            <Line
              dataKey="soglia1"
              type="monotone"
              stroke="var(--color-soglia1)"
              strokeWidth={2}
              dot={false}
            />
            <Line
              dataKey="soglia2"
              type="monotone"
              stroke="var(--color-soglia2)"
              strokeWidth={2}
              dot={false}
            />
            <Line
              dataKey="soglia3"
              type="monotone"
              stroke="var(--color-soglia3)"
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ChartContainer>
      </CardContent>
      <CardFooter>
        <div className="flex w-full items-start gap-2 text-sm">
          <div className="grid gap-2">
            <div className="flex items-center gap-2 font-medium leading-none">
              Trending up by 5.2% this month <TrendingUp className="h-4 w-4" />
            </div>
            <div className="flex items-center gap-2 leading-none text-muted-foreground">
              Showing total visitors for the last 6 months
            </div>
          </div>
        </div>
      </CardFooter>
    </Card>
  );
}
