import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import TestHarness from "./lib/TestHarness.svelte";
import { isTestPath } from "./lib/testMode";

const Root = isTestPath(window.location.pathname, import.meta.env.DEV) ? TestHarness : App;
const app = mount(Root, { target: document.getElementById("app")! });

export default app;
