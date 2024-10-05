import { createHashRouter, RouterProvider } from "react-router-dom";
import { Root } from "./pages/Root";
import "./pages/Home";
import { Home } from "./pages/Home";
import { Timeseries } from "./pages/Timeseries";
import RouteError from "./components/route-error";
import { ThemeProvider } from "./contexts/theme";

const router = createHashRouter([
  {
    path: "/",
    element: <Root />,
    errorElement: <RouteError />,
    children: [
      {
        path: "",
        element: <Home />,
      },
      {
        path: "timeseries",
        element: <Timeseries />,
      },
    ],
  },
]);

function App() {
  return (
    <main className="mx-auto flex h-screen w-screen flex-col bg-background py-8">
      <ThemeProvider>
        <RouterProvider router={router} future={{ v7_startTransition: true }} />
      </ThemeProvider>
    </main>
  );
}

export default App;
