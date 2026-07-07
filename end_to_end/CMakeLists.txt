find_package(Pytest REQUIRED)

pytest_discover_tests(
    EndToEnd
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    ENVIRONMENT "DIARIA=${CMAKE_BINARY_DIR}/src/cli/diaria"
    TRIM_FROM_NAME "^test_"
)
