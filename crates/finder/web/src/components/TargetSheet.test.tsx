import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { M31, M78 } from "../test/targets";
import { useSession } from "../useSession";
import { TargetSheet } from "./TargetSheet";

// Oracle: the OpenNGC row and the API's ordered answer/class. Text and writes exact.
afterEach(() => vi.unstubAllGlobals());

function serve(matches: unknown[] = [{ ...M31, class: "too_low" }]) {
  let target = M78;
  const fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    if (String(input).startsWith("/api/targets?")) return new Response(JSON.stringify(matches));
    if (init?.method === "PUT") target = JSON.parse(String(init.body)).target === null ? null : M31;
    return new Response(JSON.stringify({ site: null, target }));
  });
  vi.stubGlobal("fetch", fetch);
  return fetch;
}
function Sheet({ close }: { close: () => void }) {
  const { session, put } = useSession();
  return <TargetSheet open target={session.target ?? null} put={put} onClose={close} />;
}

describe("choosing the session target", () => {
  it("searches andro as typed, keeps Andromeda first with its returned class, picks and closes", async () => {
    const fetch = serve([{ ...M31, class: "too_low" }, { ...M78, class: "pointable" }]);
    const close = vi.fn();
    render(<Sheet close={close} />);
    fireEvent.change(screen.getByRole("searchbox", { name: "Search catalog" }), { target: { value: "andro" } });
    const list = await screen.findByRole("list", { name: "Matching targets" });
    const [first] = within(list).getAllByRole("button");
    expect(first!.textContent).toContain("NGC 224");
    for (const fact of ["M31", "Andromeda Galaxy", "Galaxy", "3.4", "Too low"]) expect(first!.textContent).toContain(fact);
    expect(fetch.mock.calls.some(([url]) => new URL(String(url), "http://console").searchParams.get("q") === "andro")).toBe(true);
    fireEvent.click(first!);
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    expect(fetch.mock.calls.find(([, init]) => init?.method === "PUT")?.[1]?.body).toBe('{"target":"NGC 224"}');
  });

  it("lists nothing and says nothing matches qqq", async () => {
    serve([]);
    render(<Sheet close={vi.fn()} />);
    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "qqq" } });
    expect(await screen.findByText(/Nothing matches/)).toBeTruthy();
    expect(screen.queryByRole("list", { name: "Matching targets" })).toBeNull();
  });

  it("clears the target through the session and closes", async () => {
    const fetch = serve();
    const close = vi.fn();
    render(<Sheet close={close} />);
    fireEvent.click(await screen.findByRole("button", { name: "Clear target" }));
    await waitFor(() => expect(close).toHaveBeenCalledOnce());
    expect(fetch.mock.calls.find(([, init]) => init?.method === "PUT")?.[1]?.body).toBe('{"target":null}');
  });

  it("keeps the sheet open and reports a refused edit", async () => {
    serve();
    const close = vi.fn();
    render(<TargetSheet open target={M78} put={async () => { throw new Error("Session could not be saved"); }} onClose={close} />);
    fireEvent.click(screen.getByRole("button", { name: "Clear target" }));
    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Session could not be saved");
    expect(close).not.toHaveBeenCalled();
  });
});
