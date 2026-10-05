import { mount } from "svelte";
import SortableFieldsHarness from "./SortableFieldsHarness.svelte";
import "../../src/app.css";
mount(SortableFieldsHarness, { target: document.getElementById("app")! });
