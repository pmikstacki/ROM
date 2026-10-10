import { mount } from "svelte";
import HistoryCompositionHarness from "./HistoryCompositionHarness.svelte";
import "../../src/app.css";
mount(HistoryCompositionHarness, { target: document.getElementById("app")! });
