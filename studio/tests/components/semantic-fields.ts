import { mount } from "svelte";
import Harness from "./SemanticFieldsHarness.svelte";
import "../../src/app.css";
mount(Harness, { target: document.getElementById("app")! });
