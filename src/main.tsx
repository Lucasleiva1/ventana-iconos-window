import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/global.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("No se encontró el contenedor principal de la aplicación.");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

