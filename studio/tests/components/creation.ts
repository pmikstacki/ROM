import { mount } from "svelte";
import CreationHarness from "./CreationHarness.svelte";
import "../../src/app.css";
mount(CreationHarness, { target: document.getElementById("app")! });
