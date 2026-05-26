import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Popup } from "./Popup";
import "./styles.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("popup root element missing");
}

createRoot(container).render(
  <StrictMode>
    <Popup />
  </StrictMode>,
);
