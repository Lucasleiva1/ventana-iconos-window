import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { markWindowKind } from "./windowKind";
import "./styles/global.css";

// Antes de renderizar: el CSS necesita saber qué ventana es para elegir el
// fondo. El Administrador es sólido; las demás dejan ver el escritorio.
markWindowKind();

const root = document.getElementById("root");

if (!root) {
  throw new Error("No se encontró el contenedor principal de la aplicación.");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

