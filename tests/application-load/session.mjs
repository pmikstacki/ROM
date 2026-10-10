import { awaitLoginDispatch } from "../identity/provider/login-dispatch.mjs";
const origin = "https://127.0.0.1:44389";
export function privateLoadSession(session, cookies) {
  if (
    !session ||
    session.authenticated !== true ||
    session.user_id !== "fixture-user" ||
    typeof session.csrf_token !== "string" ||
    !Array.isArray(cookies)
  )
    throw Error("actual linked load session required");
  const selected = cookies.filter(
    (value) =>
      value.name === "rom_session" &&
      value.domain === "127.0.0.1" &&
      value.secure === true &&
      value.path === "/rom-studio/",
  );
  if (
    selected.length !== 1 ||
    !/^[A-Za-z0-9_-]{1,512}$/.test(selected[0].value) ||
    !/^[A-Za-z0-9_-]{1,512}$/.test(session.csrf_token)
  )
    throw Error("private bounded load session required");
  return {
    cookie: `rom_session=${selected[0].value}`,
    csrf: session.csrf_token,
  };
}
export async function loginForLoad(page, context, password) {
  if (
    typeof password !== "string" ||
    password.length < 1 ||
    password.length > 256
  )
    throw Error("private synthetic login input required");
  const callback = page.waitForResponse(
    (response) => {
      const url = new URL(response.url());
      return (
        url.origin === origin &&
        url.pathname === "/rom-studio/auth/callback/authentik"
      );
    },
    { timeout: 20000 },
  );
  callback.catch(() => {});
  await page.goto(origin + "/rom-studio/auth/login/authentik", {
    timeout: 15000,
  });
  const input = page
    .locator("ak-stage-identification")
    .locator('input[name="uidField"]');
  const dispatch = await awaitLoginDispatch(
    input.waitFor({ state: "visible", timeout: 20000 }),
    callback,
  );
  if (dispatch.kind === "form") {
    await input.fill("akadmin");
    await page.getByRole("button", { name: /log in|continue/i }).click();
    const passwordStage = page.locator("ak-stage-password");
    await passwordStage.waitFor({ state: "visible", timeout: 10000 });
    await passwordStage.locator('input[name="password"]').fill(password);
    await passwordStage.locator('input[name="password"]').press("Enter");
  }
  const response =
    dispatch.kind === "callback" ? dispatch.response : await callback;
  if (response.status() !== 303) throw Error("actual load login rejected");
  await page.waitForURL(
    (url) => url.origin === origin && url.pathname === "/rom-studio/",
    { timeout: 10000 },
  );
  const sessionResponse = await context.request.get(
    origin + "/rom-studio/auth/session",
    { timeout: 5000 },
  );
  if (sessionResponse.status() !== 200)
    throw Error("actual load session denied");
  return privateLoadSession(
    await sessionResponse.json(),
    await context.cookies(origin + "/rom-studio/"),
  );
}
