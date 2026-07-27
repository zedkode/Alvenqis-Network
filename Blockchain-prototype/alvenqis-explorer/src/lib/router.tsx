import {
  AnchorHTMLAttributes,
  createContext,
  MouseEvent,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";

interface NavigateOptions {
  replace?: boolean;
}

interface RouterContextValue {
  pathname: string;
  navigate: (to: string, options?: NavigateOptions) => void;
}

const RouterContext = createContext<RouterContextValue | null>(null);
const ParamsContext = createContext<Record<string, string | undefined>>({});

function currentPathname() {
  return window.location.pathname.replace(/\/+$/, "") || "/";
}

export function BrowserRouter({ children }: { children: ReactNode }) {
  const [pathname, setPathname] = useState(currentPathname);

  useEffect(() => {
    const handlePopState = () => setPathname(currentPathname());
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  const navigate = useCallback((to: string, options: NavigateOptions = {}) => {
    const url = new URL(to, window.location.origin);
    const nextUrl = `${url.pathname}${url.search}${url.hash}`;
    if (options.replace) {
      window.history.replaceState({}, "", nextUrl);
    } else {
      window.history.pushState({}, "", nextUrl);
    }
    setPathname(url.pathname.replace(/\/+$/, "") || "/");
    window.scrollTo({ top: 0, behavior: "smooth" });
  }, []);

  const value = useMemo(() => ({ pathname, navigate }), [navigate, pathname]);
  return <RouterContext.Provider value={value}>{children}</RouterContext.Provider>;
}

function useRouter() {
  const context = useContext(RouterContext);
  if (!context) {
    throw new Error("Router hooks require BrowserRouter.");
  }
  return context;
}

interface LinkProps extends Omit<AnchorHTMLAttributes<HTMLAnchorElement>, "href"> {
  to: string;
}

export function Link({ to, onClick, target, ...props }: LinkProps) {
  const { navigate } = useRouter();

  function handleClick(event: MouseEvent<HTMLAnchorElement>) {
    onClick?.(event);
    if (
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey ||
      target === "_blank"
    ) {
      return;
    }

    const url = new URL(to, window.location.origin);
    if (url.origin !== window.location.origin) {
      return;
    }

    event.preventDefault();
    navigate(`${url.pathname}${url.search}${url.hash}`);
  }

  return <a {...props} href={to} target={target} onClick={handleClick} />;
}

export function NavLink({ className = "", to, ...props }: LinkProps) {
  const { pathname } = useRouter();
  const active = pathname === to || (to !== "/dashboard" && pathname.startsWith(`${to}/`));
  return (
    <Link
      {...props}
      className={`${className} ${active ? "active" : ""}`.trim()}
      to={to}
      aria-current={active ? "page" : undefined}
    />
  );
}

export function useNavigate() {
  return useRouter().navigate;
}

export function usePathname() {
  return useRouter().pathname;
}

export function RouteParamsProvider({
  children,
  params,
}: {
  children: ReactNode;
  params: Record<string, string | undefined>;
}) {
  return <ParamsContext.Provider value={params}>{children}</ParamsContext.Provider>;
}

export function useParams() {
  return useContext(ParamsContext);
}
