import { mount } from "svelte";
import App from "../../studio/src/App.svelte";
import "../../studio/src/app.css";
import { registerRenderer } from "../../studio/src/lib/renderers/registry.ts";
import TicketCode from "./TicketCode.svelte";
// Explicit demo extension. The registry key is a codec identity, never a Resource kind.
registerRenderer({ name: "demo-ticket-code", version: 1 }, TicketCode);
mount(App, { target: document.getElementById("app")! });
