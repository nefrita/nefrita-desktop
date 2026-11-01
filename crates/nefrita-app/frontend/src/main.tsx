import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";

const appContainer = document.getElementById("app-container");
if (appContainer === null) {
  throw new Error("The page has no app container");
}

createRoot(appContainer).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
