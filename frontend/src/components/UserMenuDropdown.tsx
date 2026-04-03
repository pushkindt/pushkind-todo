import type { UserMenuItem } from "../lib/models";

function menuItemIconClass(item: UserMenuItem) {
  if (item.iconClass) {
    return item.iconClass;
  }

  if (item.name === "Главная" || item.name === "Домой") {
    return "bi bi-house";
  }

  if (item.name === "Настройки") {
    return "bi bi-gear";
  }

  return "bi bi-grid";
}

const LOGOUT_ITEM_NAMES = new Set([
  "logout",
  "log out",
  "sign out",
  "signout",
  "выйти",
]);

function normalizedPath(url: string) {
  try {
    return new URL(url, "https://pushkind.local").pathname.replace(/\/+$/, "");
  } catch {
    return undefined;
  }
}

function isLogoutItem(item: UserMenuItem, logoutAction: string) {
  const normalizedName = item.name.trim().toLowerCase();
  if (LOGOUT_ITEM_NAMES.has(normalizedName)) {
    return true;
  }

  return normalizedPath(item.url) === normalizedPath(logoutAction);
}

type UserMenuDropdownProps = {
  currentUserEmail: string;
  localItems: UserMenuItem[];
  fetchedItems: UserMenuItem[];
  logoutAction: string;
};

export function UserMenuDropdown({
  currentUserEmail,
  localItems,
  fetchedItems,
  logoutAction,
}: UserMenuDropdownProps) {
  const visibleLocalItems = localItems.filter(
    (item) => !isLogoutItem(item, logoutAction),
  );
  const visibleFetchedItems = fetchedItems.filter(
    (item) => !isLogoutItem(item, logoutAction),
  );
  const hasNavigationItems =
    visibleLocalItems.length > 0 || visibleFetchedItems.length > 0;

  return (
    <div className="dropdown-center">
      <button
        className="btn btn-link nav-link align-items-center text-muted dropdown-toggle"
        type="button"
        data-bs-toggle="dropdown"
        aria-expanded="false"
      >
        <i className="bi bi-person-circle fs-4" />
      </button>
      <ul className="dropdown-menu dropdown-menu-end">
        <li>
          <h6 className="dropdown-header">{currentUserEmail}</h6>
        </li>
        {hasNavigationItems ? (
          <li>
            <hr className="dropdown-divider" />
          </li>
        ) : null}
        {visibleLocalItems.map((item) => (
          <li key={`local-${item.url}-${item.name}`}>
            <a className="dropdown-item icon-link" href={item.url}>
              <i className={`${menuItemIconClass(item)} mb-2`} />
              {item.name}
            </a>
          </li>
        ))}
        {visibleFetchedItems.map((item) => (
          <li key={`fetched-${item.url}-${item.name}`}>
            <a className="dropdown-item icon-link" href={item.url}>
              <i className={`${menuItemIconClass(item)} mb-2`} />
              {item.name}
            </a>
          </li>
        ))}
        <li>
          <form method="POST" action={logoutAction}>
            <button type="submit" className="dropdown-item icon-link">
              <i className="bi bi-box-arrow-right mb-2" />
              Выйти
            </button>
          </form>
        </li>
      </ul>
    </div>
  );
}
