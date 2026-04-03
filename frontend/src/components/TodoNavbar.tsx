import type { ReactNode } from "react";

import { UserMenuDropdown } from "./UserMenuDropdown";
import type { NavigationItem, UserMenuItem } from "../lib/models";

type TodoNavbarProps = {
  navigation: NavigationItem[];
  currentUserEmail: string;
  homeUrl: string;
  localMenuItems: UserMenuItem[];
  fetchedMenuItems: UserMenuItem[];
  search?: ReactNode;
};

export function TodoNavbar({
  navigation,
  currentUserEmail,
  homeUrl,
  localMenuItems,
  fetchedMenuItems,
  search,
}: TodoNavbarProps) {
  return (
    <div className="container pt-2">
      <nav className="navbar navbar-expand-sm bg-body-tertiary">
        <div className="container-fluid">
          <a className="navbar-brand" href="/">
            ToDo
          </a>
          <button
            className="navbar-toggler"
            type="button"
            data-bs-toggle="collapse"
            data-bs-target="#todo-foundation-navbar"
            aria-controls="todo-foundation-navbar"
            aria-expanded="false"
            aria-label="Toggle navigation"
          >
            <span className="navbar-toggler-icon" />
          </button>
          <div className="collapse navbar-collapse" id="todo-foundation-navbar">
            <ul className="navbar-nav me-auto">
              {navigation.map((item) => (
                <li className="nav-item" key={item.url}>
                  <a className="nav-link" href={item.url}>
                    {item.name}
                  </a>
                </li>
              ))}
            </ul>
            {search ? <div className="todo-navbar-search">{search}</div> : null}
          </div>
          <div className="ms-sm-2">
            <UserMenuDropdown
              currentUserEmail={currentUserEmail}
              localItems={[{ name: "Домой", url: homeUrl }, ...localMenuItems]}
              fetchedItems={fetchedMenuItems}
              logoutAction="/logout"
            />
          </div>
        </div>
      </nav>
    </div>
  );
}
