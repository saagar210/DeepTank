import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

const style = document.createElement("style");
style.textContent = `
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body, #root { width: 100%; height: 100%; overflow: hidden; }
  @keyframes fadeIn {
    from { opacity: 0; transform: translateX(20px); }
    to { opacity: 1; transform: translateX(0); }
  }
`;
document.head.appendChild(style);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
