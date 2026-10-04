import { mount } from "svelte";
import RendererRegressionHarness from "./RendererRegressionHarness.svelte";
import "../../src/app.css";

mount(RendererRegressionHarness, { target: document.getElementById("app")! });
