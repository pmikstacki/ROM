import { mount } from "svelte";
import ConversationLayoutFixture from "./fixtures/ConversationLayoutFixture.svelte";
import "../../src/app.css";
mount(ConversationLayoutFixture, { target: document.getElementById("app")! });
