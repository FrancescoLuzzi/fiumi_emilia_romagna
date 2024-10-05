"use client";

import { Link, useLocation } from "react-router-dom";
import { getTimeSeries } from "@/api/timeseries";
import { useEffect, useState } from "react";
import { TimeSeries } from "@/types/timeseries";
import { Station } from "@/types/station";
import { TrendingUp } from "lucide-react";
import { Area, AreaChart, CartesianGrid, XAxis } from "recharts";

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
  const [timeSeries, setTimeSeries] = useState<TimeSeries>([]);
  useEffect(() => {
    console.log(station);
    getTimeSeries(station).then(setTimeSeries);
  }, [station]);
  return (
    <div>
      <Link to="/">GO BACK</Link>
      <ul>
        {timeSeries.map((x) => (
          <li>
            {x.t}:{x.v}
          </li>
        ))}
      </ul>
    </div>
  );
};
